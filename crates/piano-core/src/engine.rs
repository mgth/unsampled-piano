//! Polyphonic engine: voice management, events, pedals and block rendering.
//!
//! Each key is a [`Voice`]: 1–3 stiff strings coupled through the bridge
//! (double decay, beating), struck by a felt hammer through the passive
//! contact solver, damped by a felt damper driven by the key and the pedals.
//! The bridge forces, grouped by bridge region, drive the modal soundboard,
//! which radiates to two microphones and moves the bridge under every other
//! string (sympathetic resonance).

use std::sync::Arc;

use crate::board::{BoardBank, PARTS, PROBES};
use crate::denormal::DenormalGuard;
use crate::instrument::{region_of, Instrument, MICS, REGIONS};
use crate::live::LiveParam;
use crate::modal::ModalBank;
use crate::params::PianoModel;
use crate::voice::{Voice, RING};

pub use crate::voice::MAX_MODES;
/// Internal processing granularity; also the fixed delay (samples) of the
/// bridge displacement fed back to the strings.
pub const SUB_BLOCK: usize = 64;
/// Output scaling without soundboard: bridge force (N) to full scale.
/// Lowered by 2.7 dB with the faster hammer law (docs/brightness.md), same
/// loudness for recorded performances; raised by 6 dB with the pressure
/// gain below.
const FORCE_GAIN: f32 = 1.0 / 300.0;
/// Output scaling with soundboard: pressure at the mics (Pa) to full scale.
/// Before `OUTPUT_LEVEL_DB`, performances sat at −33 to −37 LUFS (they sat
/// at −39 to −43), a ten-note fortissimo chord peaked at −6 dBFS, and the
/// loudest case tried, 16-note fff chords four times over with the pedal
/// down, at −2 dBFS (the engine's microphones; +2 dBFS with the close ones).
const PRESSURE_GAIN: f32 = 1.0 / 15.0;
/// Voices whose bridge force falls below ≈ 1e-5·240 N (−100 dB of a loud
/// note) are freed, unless their strings are free to resonate.
const SILENCE_ENERGY: f32 = (1e-5 / FORCE_GAIN) * (1e-5 / FORCE_GAIN);
/// Below this mic energy and without input, the soundboard sleeps.
const BOARD_SILENCE: f32 = (1e-6 / PRESSURE_GAIN) * (1e-6 / PRESSURE_GAIN);
/// Level at output_gain_db 0 (dB), on top of the gains above: performances
/// sit near −20 LUFS and peak a few dB under full scale, as unprocessed
/// recordings of real grands do (SMD: −20 to −23 LUFS, peaks at −2 dBFS);
/// they sat 9 dB lower, far under the system's other sounds. Loud treble
/// chords can pass full scale: the output limiter catches them
/// (docs/limiter.md).
const OUTPUT_LEVEL_DB: f64 = 9.0;
/// Time constant (s) of the output gain smoothing.
const GAIN_SMOOTHING: f64 = 0.02;
/// Pending frame knocks (keybed, pedal) at a time.
const MAX_KNOCKS: usize = 32;

/// A half-sine force pulse into the board at one region.
#[derive(Clone, Copy, Default)]
struct Knock {
    region: usize,
    pos: usize,
    len: usize,
    peak: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8, velocity: u8 },
    Control { cc: u8, value: u8 },
    AllNotesOff,
    /// A live parameter change ([`LiveParam`]), applied at its frame.
    Param { id: LiveParam, value: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedEvent {
    /// Offset in samples from the start of the block.
    pub frame: u32,
    pub event: Event,
}

#[derive(Clone, Copy, Debug)]
pub struct EngineConfig {
    pub sample_rate: f64,
    pub max_block: usize,
    pub polyphony: usize,
    pub model: PianoModel,
    /// Threads rendering the voices (the audio thread included); 1: none
    /// started. The output does not depend on it.
    pub threads: usize,
    /// Run by each worker thread when it starts, with `max_block` and
    /// `sample_rate`: the host gives it the audio thread's real-time
    /// priority (a preempted worker makes the audio callback wait).
    pub worker_init: Option<fn(usize, f64)>,
    /// CPU quality: parts of the model left out on smaller machines, applied
    /// to every instrument built ([`Quality::apply`]).
    pub quality: Quality,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            max_block: 128,
            polyphony: 128,
            model: PianoModel::default(),
            threads: 1,
            worker_init: None,
            quality: Quality::Full,
        }
    }
}

/// CPU quality levels (docs/performance.md, "Compromises for smaller
/// configurations"). The preset keeps its values: a level only changes the
/// model the instrument is built from and the engine's shortcuts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality {
    /// The whole model with none of the shortcuts that change the signal,
    /// even those judged inaudible ([`Shortcuts`]): every partial below
    /// 0.45·fs, sympathy and phantom partials on every one, every string
    /// hearing the bridge, silences 200 dB down, twice the contact steps.
    Overkill,
    /// The whole model, with shortcuts judged inaudible.
    #[default]
    Full,
    /// Sympathetic strings below 2 kHz, phantom partials up to C3, 96
    /// voices: about a third less in dense music.
    Balanced,
    /// No sympathetic resonance, no phantom partials, no frame or rim, one
    /// polarisation: about a third of Full (AVX2 laptops, ARM).
    Light,
}

impl Quality {
    pub const ALL: [Quality; 4] = [Quality::Overkill, Quality::Full, Quality::Balanced, Quality::Light];

    /// The model an instrument is built from at this level.
    pub fn apply(self, model: &PianoModel) -> PianoModel {
        let mut m = *model;
        match self {
            Quality::Overkill => {
                m.sympathetic.max_freq = f64::INFINITY;
                m.nonlinear.max_note = 108;
                m.nonlinear.max_partial_hz = f64::INFINITY;
                m.nonlinear.max_long_hz = f64::INFINITY;
            }
            Quality::Full => {}
            Quality::Balanced => {
                m.sympathetic.max_freq = m.sympathetic.max_freq.min(2000.0);
                m.nonlinear.max_note = m.nonlinear.max_note.min(48);
            }
            Quality::Light => {
                m.sympathetic.enabled = false;
                m.nonlinear.enabled = false;
                m.frame.enabled = false;
                m.rim.enabled = false;
                m.polarization.enabled = false;
            }
        }
        m
    }

    /// Caps on the strings' resolution at this level.
    pub fn resolution(self) -> crate::note_model::Resolution {
        match self {
            Quality::Overkill => crate::note_model::Resolution::UNCAPPED,
            _ => crate::note_model::Resolution::DEFAULT,
        }
    }

    /// The instrument for `model` at this level (≈ 0.1 s; Overkill longer):
    /// not for the audio thread.
    pub fn instrument(self, model: &PianoModel, sample_rate: f64) -> Instrument {
        Instrument::with_resolution(self.apply(model), sample_rate, self.resolution())
    }

    /// The engine's shortcuts at this level.
    pub fn shortcuts(self) -> Shortcuts {
        match self {
            Quality::Overkill => Shortcuts::NONE,
            _ => Shortcuts::DEFAULT,
        }
    }

    /// Voices at this level, at most `polyphony`.
    pub fn polyphony(self, polyphony: usize) -> usize {
        match self {
            Quality::Balanced => polyphony.min(96),
            _ => polyphony,
        }
    }

    /// Lowercase name, as in play.toml.
    pub fn name(self) -> &'static str {
        match self {
            Quality::Overkill => "overkill",
            Quality::Full => "full",
            Quality::Balanced => "balanced",
            Quality::Light => "light",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|q| q.name().eq_ignore_ascii_case(name))
    }
}

/// Shortcuts of the engine that change the signal, judged inaudible:
/// thresholds below which a part stops being computed. [`Quality::Overkill`]
/// puts them 200 dB below a loud note (56 dB under 24-bit resolution) and
/// lets every string hear the bridge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shortcuts {
    /// A voice is freed below this bridge-force energy (N²·gain).
    pub voice_silence: f32,
    /// The soundboard, frame and rim sleep below this mic energy.
    pub board_silence: f32,
    /// A sympathetic bank at rest is left alone below this energy (N²).
    pub sym_silence: f32,
    /// Phantom partials are skipped below this tension modulation (N).
    pub nl_silence: f32,
    /// Strings damped beyond this engagement no longer hear the bridge.
    pub deaf_above: f64,
    /// Every string listens to the bridge, damped or not, pedal up or down,
    /// and is never freed; otherwise only strings with their damper lifted
    /// get a listening voice.
    pub listen_all: bool,
    /// A struck string drops its partials once they fall this far below a
    /// loud note's bridge force (energy ratio); 0: never.
    pub drop_partials: f32,
}

impl Shortcuts {
    pub const DEFAULT: Self = Self {
        voice_silence: SILENCE_ENERGY,
        board_silence: BOARD_SILENCE,
        sym_silence: 1e-12,
        nl_silence: 1e-4,
        deaf_above: 0.99,
        listen_all: false,
        drop_partials: 1e-2,
    };
    pub const NONE: Self = Self {
        voice_silence: SILENCE_ENERGY * 1e-10,
        board_silence: BOARD_SILENCE * 1e-10,
        sym_silence: 1e-22,
        nl_silence: 1e-14,
        deaf_above: f64::INFINITY,
        listen_all: true,
        drop_partials: 0.0,
    };
}

pub struct Engine {
    cfg: EngineConfig,
    shortcuts: Shortcuts,
    inst: Arc<Instrument>,
    /// The instrument's model with the live parameters applied
    /// ([`LiveParam`]); its other fields always equal the instrument's.
    live: PianoModel,
    /// Live parameters before the musician's controls: a live control is
    /// applied to these, so that moving it never compounds.
    base: PianoModel,
    /// Output gain, smoothed towards `live.output_gain_db`.
    gain: f32,
    voices: Vec<Voice>,
    /// Pedal positions, 0–1.
    sustain: f64,
    /// Share of the damped strings' load the board feels (follows the
    /// dampers lifted by the pedal), and the value its banks hold.
    board_load: f64,
    board_load_set: f64,
    /// The dampers are lifted: the sympathetic banks run the brakes of every
    /// free string ([`crate::params::SympatheticLaw::brake`]).
    sym_free: bool,
    soft: f64,
    sostenuto_down: bool,
    /// Middle pedal down while it works a practice mute (`mute_felt` > 0).
    mute_down: bool,
    /// Sample time of the event being handled.
    now: u64,
    /// Per key: down, or the time it was released (None: at rest since
    /// long enough or never played), for the upright's repetition.
    key_down: [bool; 128],
    key_up_at: [Option<u64>; 128],
    /// Time of each key's last note-on (duplicates, see DUPLICATE_S).
    key_down_at: [u64; 128],
    /// Note-ons ignored as duplicates of the one before.
    doubled: u32,
    /// Bridge force per region for the current block: primary (hammer
    /// response) and sympathetic.
    region: Vec<Vec<f32>>,
    region_sym: Vec<Vec<f32>>,
    /// The strings' part of each microphone (zone 0), the board's forced
    /// response at their frequencies (`soundboard.forced_radiation`):
    /// empty when off.
    forced_mics: Vec<Vec<f32>>,
    /// Force on the agraffes per region (primary strings), for the frame.
    region_agraffe: Vec<Vec<f32>>,
    frame: Option<BoardBank>,
    frame_awake: bool,
    /// Rim modes and the board's edge-force projections that drive them.
    rim: Option<crate::structure::RimBank>,
    rim_force: Vec<Vec<f32>>,
    rim_awake: bool,
    /// Soundboard driven by the primary forces (radiates and moves the
    /// bridge for the strings), and by the sympathetic ones (radiates only).
    board: Option<BoardBank>,
    board_sym: Option<BoardBank>,
    /// The strings' rear lengths, one bank per board region
    /// (`sympathetic.rear`): they listen to the bridge there and push on
    /// the sympathetic board, whatever the dampers do.
    rear: Vec<ModalBank>,
    /// The low modes driven at each key (Instrument::low) and their
    /// profile inputs for the current block. The sympathetic forces keep
    /// the regions: a diffuse halo, where the exact drive changes nothing
    /// audible and would double the cost.
    board_low: Option<BoardBank<{ crate::board::LOW_INPUTS }>>,
    basis: Vec<Vec<f32>>,
    /// The modes that only radiate (`sympathetic.bridge_cut`: those above
    /// its cut), driven by the struck and the sympathetic forces together
    /// (`region_all`); `board` and `board_sym` then stop under the cut.
    board_top: Option<BoardBank>,
    region_all: Vec<Vec<f32>>,
    board_sym_awake: bool,
    board_awake: bool,
    /// Bridge velocity per region (this sub-block) and its last sample; the
    /// acceleration history (ring by absolute sample) and its delayed copy
    /// fed to the strings.
    disp: Vec<Vec<f32>>,
    last_vel: [f32; REGIONS],
    disp_ring: Vec<Vec<f32>>,
    disp_in: Vec<Vec<f32>>,
    /// Absolute sample index of the next sub-block.
    clock: u64,
    /// The microphones' pressure over the sub-block (the first `n_mics`),
    /// the action's and the body's two sides, and the stereo mix for
    /// [`Self::process`].
    mic_buf: [Vec<f32>; MICS],
    noise_buf: [Vec<f32>; 2],
    stereo: [Vec<f32>; 2],
    /// Microphones of the set-up; per mic: its (left, right) gains in the
    /// stereo mix, its gain on its own channel, and its share of the
    /// action's and the body's two sides (by its pan).
    n_mics: usize,
    /// Mic slots the board's banks fill: the mics, or their delay zones
    /// ([`crate::board::Zones`]) folded into them before the pressure.
    n_slots: usize,
    fold_all: Option<crate::board::ZoneFold>,
    fold_board: Option<crate::board::ZoneFold>,
    mix: [(f32, f32); MICS],
    own_gain: [f32; MICS],
    noise_w: [(f32, f32); MICS],
    /// How far each microphone hears the noises (crate::mics::noise_paths):
    /// gain and delay (samples), through a history of the two sides.
    noise_gain: [f32; MICS],
    noise_delay: [usize; MICS],
    noise_hist: [Vec<f32>; 2],
    noise_pos: usize,
    per_mic: bool,
    knocks: [Knock; MAX_KNOCKS],
    /// The action's knock, heard directly, and the decay time it was set to.
    action: crate::action::ActionNoise,
    action_time: f64,
    /// The body's halo after each strike (crate::body).
    body: crate::body::BodyBank,
    room: Option<crate::room::Room>,
    /// Damper engagement below which a landing damper counts as an impact.
    sustain_prev: f64,
    /// Last summed mic outputs, for the differentiator.
    mic_last: [f32; MICS],
    /// The board's own part of the mics and its pressure at the probes,
    /// for the room (acceleration sums, then pressure), and their
    /// differentiators' last values.
    board_mics: [Vec<f32>; MICS],
    probes: [Vec<f32>; PROBES],
    board_last: [f32; MICS + PROBES],
    pool: Option<crate::par::Pool>,
    /// Mic outputs of the frame and of the sympathetic board when they run
    /// on the pool, summed after the board's.
    side_out: [[Vec<f32>; MICS]; 2],
    /// Indices of the voices rendered in the current segment.
    busy: Vec<usize>,
}

/// Lets the pool's workers reach disjoint voices, banks and buffers.
struct Ptr<T>(*mut T);
// SAFETY: each item of a pool run touches only its own target.
unsafe impl<T> Sync for Ptr<T> {}

/// Segments with fewer voices are rendered on the audio thread alone.
const PARALLEL_MIN_VOICES: usize = 4;

impl Engine {
    /// Builds the instrument (≈ 0.5 s): not for the audio thread.
    pub fn new(cfg: EngineConfig) -> Self {
        Self::with_instrument(cfg, Arc::new(cfg.quality.instrument(&cfg.model, cfg.sample_rate)))
    }

    /// Engine on a prebuilt (shared) instrument, built by
    /// `cfg.quality.instrument`; `cfg.model` is ignored.
    pub fn with_instrument(cfg: EngineConfig, inst: Arc<Instrument>) -> Self {
        assert_eq!(inst.sample_rate, cfg.sample_rate);
        let cfg = EngineConfig { polyphony: cfg.quality.polyphony(cfg.polyphony), ..cfg };
        let shortcuts = cfg.quality.shortcuts();
        let capacity = inst.table.capacity();
        let action_time = inst.model.noise.action_time;
        let body_law = inst.model.body;
        // Only an instrument whose bridge rocks pays for its third output.
        let rocking = inst.low.as_ref().is_some_and(|l| l.rocking);
        // Nor for the board's forced response (soundboard.forced_radiation).
        let forced = inst.table.notes().iter().map(|n| n.forced.len()).max().unwrap_or(0);
        let voices = (0..cfg.polyphony)
            .map(|_| {
                let mut v = Voice::with_capacity(cfg.sample_rate, cfg.max_block, capacity);
                v.set_shortcuts(&shortcuts);
                if rocking {
                    v.reserve_rocking();
                }
                if forced > 0 {
                    v.reserve_forced(forced);
                }
                v
            })
            .collect();
        let rim_couple = inst.rim.as_ref().map_or(&[][..], |r| &r.couple[..]);
        // The low modes apart when they are driven at each key; the rim's
        // projections follow their modes.
        let n_low = inst.low.as_ref().map_or(0, |l| l.n);
        let n_basis = if inst.low.is_some() { crate::board::LOW_INPUTS } else { 0 };
        let (low_couple, high_couple) = rim_couple.split_at(n_low.min(rim_couple.len()));
        let low_modes = inst.low.as_ref().zip(inst.board.as_ref()).map(|(l, m)| l.modes(m));
        // The modes that move the bridge under the strings (all of them
        // without `sympathetic.bridge_cut`), and those that only radiate.
        let n_heard = inst.board.as_ref().map_or(0, |m| crate::note_model::heard_modes(&inst.model, m, inst.low.as_ref().map(|l| l.n)));
        let (high_couple, top_couple) = high_couple.split_at((n_heard - n_low).min(high_couple.len()));
        let board = inst.board.as_ref().map(|m| BoardBank::with_extra(cfg.sample_rate, &m[n_low..n_heard], high_couple, SUB_BLOCK));
        let board_top = inst.board.as_ref().filter(|m| n_heard < m.len()).map(|m| BoardBank::with_extra(cfg.sample_rate, &m[n_heard..], top_couple, SUB_BLOCK));
        let board_low = low_modes.as_ref().map(|m| BoardBank::with_extra(cfg.sample_rate, m, low_couple, SUB_BLOCK));
        let frame = inst.frame.as_ref().map(|m| BoardBank::new(cfg.sample_rate, m, SUB_BLOCK));
        let rim = inst.rim.as_ref().map(|r| crate::structure::RimBank::new(r, cfg.sample_rate));
        let n_proj = inst.rim.as_ref().map_or(0, |r| r.projections.len());
        let board_sym = inst
            .board
            .as_ref()
            .filter(|_| inst.model.sympathetic.enabled)
            .map(|m| BoardBank::new(cfg.sample_rate, &m[..n_heard], SUB_BLOCK));
        // The radiation's zones: each bank lets the board sleep only once
        // all the first microphone's slots are quiet.
        let (mut board, mut board_low, mut board_sym, mut board_top) = (board, board_low, board_sym, board_top);
        if let Some(z) = &inst.zones {
            board.iter_mut().chain(board_sym.iter_mut()).chain(board_top.iter_mut()).for_each(|b| b.set_zones(z));
            board_low.iter_mut().for_each(|b| b.set_zones(z));
        }
        let rear: Vec<ModalBank> = inst
            .rear
            .iter()
            .flatten()
            .map(|m| {
                let mut b = ModalBank::new(cfg.sample_rate, m.len().max(1), SUB_BLOCK);
                b.set_modes(m.iter().copied());
                b
            })
            .collect();
        let room = match (inst.model.soundboard, inst.board.is_some()) {
            (Some(sb), true) if inst.model.room.enabled => {
                Some(crate::room::Room::new(&inst.model.room, &sb, cfg.sample_rate))
            }
            _ => None,
        };
        // The listening set-up's microphones (without a board: the bridge
        // force on a pair).
        let mics = inst.model.soundboard.map_or(crate::mics::Mics::pair((0.0, 0.0, 0.0), (0.0, 0.0, 0.0)), |s| s.mics);
        let mics = if mics.is_empty() { crate::mics::Mics::pair((0.0, 0.0, 0.0), (0.0, 0.0, 0.0)) } else { mics };
        let per_mic = inst.model.soundboard.is_some_and(|s| s.outputs_per_mic);
        let noise_paths = inst.noise_paths.clone();
        // (A delay past the bound would size the history without one.)
        let max_delay = (crate::mics::MAX_NOISE_DELAY * cfg.sample_rate).round() as usize;
        let noise_path = |m: usize| noise_paths.get(m).map_or((1.0, 0), |&(g, d)| (g, d.min(max_delay)));
        let noise_span = (0..MICS).map(|m| noise_path(m).1).max().unwrap_or(0);
        let zones = inst.zones.filter(|z| z.mics == mics.len());
        let live = inst.model;
        let base = inst.source;
        let mut e = Self {
            cfg,
            shortcuts,
            inst,
            live,
            base,
            gain: db_to_gain(live.output_gain_db + OUTPUT_LEVEL_DB),
            voices,
            sustain: 0.0,
            board_load: 1.0,
            board_load_set: 1.0,
            sym_free: false,
            soft: 0.0,
            sostenuto_down: false,
            mute_down: false,
            now: 0,
            key_down: [false; 128],
            key_up_at: [None; 128],
            key_down_at: [0; 128],
            doubled: 0,
            region: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            region_sym: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            forced_mics: if forced > 0 { vec![vec![0.0; SUB_BLOCK]; mics.len()] } else { Vec::new() },
            region_agraffe: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            frame,
            frame_awake: false,
            rim,
            rim_force: vec![vec![0.0; SUB_BLOCK]; n_proj],
            rim_awake: false,
            board,
            board_sym,
            rear,
            board_low,
            basis: vec![vec![0.0; SUB_BLOCK]; n_basis],
            region_all: vec![vec![0.0; SUB_BLOCK]; if board_top.is_some() { REGIONS } else { 0 }],
            board_top,
            board_sym_awake: false,
            board_awake: false,
            disp: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            last_vel: [0.0; REGIONS],
            disp_ring: vec![vec![0.0; RING]; REGIONS],
            disp_in: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            clock: 0,
            stereo: std::array::from_fn(|_| vec![0.0; cfg.max_block]),
            mic_buf: std::array::from_fn(|_| vec![0.0; SUB_BLOCK]),
            noise_buf: std::array::from_fn(|_| vec![0.0; SUB_BLOCK]),
            n_mics: mics.len(),
            n_slots: zones.map_or(mics.len(), |z| z.slots()),
            fold_all: zones.map(|z| crate::board::ZoneFold::new(z, SUB_BLOCK)),
            fold_board: zones.map(|z| crate::board::ZoneFold::new(z, SUB_BLOCK)),
            mix: std::array::from_fn(|m| mics.as_slice().get(m).map_or((0.0, 0.0), |mic| {
                let (g, (l, r)) = (10f64.powf(mic.gain_db / 20.0), mic.pan_gains());
                ((g * l) as f32, (g * r) as f32)
            })),
            own_gain: std::array::from_fn(|m| mics.as_slice().get(m).map_or(0.0, |mic| 10f64.powf(mic.gain_db / 20.0) as f32)),
            noise_w: std::array::from_fn(|m| mics.as_slice().get(m).map_or((0.0, 0.0), |mic| {
                let p = mic.pan.clamp(-1.0, 1.0) as f32;
                (0.5 * (1.0 - p), 0.5 * (1.0 + p))
            })),
            noise_gain: std::array::from_fn(|m| noise_path(m).0 as f32),
            noise_delay: std::array::from_fn(|m| noise_path(m).1),
            noise_hist: std::array::from_fn(|_| vec![0.0; (noise_span + SUB_BLOCK).next_power_of_two()]),
            noise_pos: 0,
            per_mic,
            knocks: [Knock::default(); MAX_KNOCKS],
            action: crate::action::ActionNoise::new(cfg.sample_rate, action_time),
            action_time,
            body: crate::body::BodyBank::new(cfg.sample_rate, &body_law),
            room,
            sustain_prev: 0.0,
            mic_last: [0.0; MICS],
            board_mics: std::array::from_fn(|_| vec![0.0; SUB_BLOCK]),
            probes: std::array::from_fn(|_| vec![0.0; SUB_BLOCK]),
            board_last: [0.0; MICS + PROBES],
            pool: Self::pool_for(&cfg),
            side_out: std::array::from_fn(|_| std::array::from_fn(|_| vec![0.0; SUB_BLOCK])),
            busy: Vec::with_capacity(cfg.polyphony),
        };
        // Strings without dampers always resonate.
        if e.sympathy() {
            for note in e.inst.model.damper.highest_note.saturating_add(1)..=108 {
                e.add_listener(note);
            }
        }
        e
    }

    /// The worker pool `cfg` asks for (`threads` > 1), each worker started
    /// with `worker_init`.
    pub fn pool_for(cfg: &EngineConfig) -> Option<crate::par::Pool> {
        (cfg.threads > 1).then(|| {
            let (init, mb, sr) = (cfg.worker_init, cfg.max_block, cfg.sample_rate);
            crate::par::Pool::with_init(cfg.threads, move || {
                if let Some(f) = init {
                    f(mb, sr)
                }
            })
        })
    }

    /// Lends the engine a worker pool (or takes it back with `None`) and
    /// returns the one it had: a host running several engines shares one
    /// pool between them. Real-time safe (moves, no allocation).
    pub fn swap_pool(&mut self, pool: Option<crate::par::Pool>) -> Option<crate::par::Pool> {
        std::mem::replace(&mut self.pool, pool)
    }

    /// Replaces the shortcuts set by the quality level (tests, comparisons).
    pub fn set_shortcuts(&mut self, s: Shortcuts) {
        self.shortcuts = s;
        for v in &mut self.voices {
            v.set_shortcuts(&s);
        }
    }

    pub fn instrument(&self) -> &Arc<Instrument> {
        &self.inst
    }

    /// The live parameters as they stand (the musician's controls applied).
    pub fn live_model(&self) -> &PianoModel {
        &self.live
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    /// Note-ons ignored so far as duplicates of the one before (a keyboard
    /// reaching the engine twice: two channels or two MIDI connections).
    pub fn doubled_notes(&self) -> u32 {
        self.doubled
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Voices actually sounding: struck or set ringing, not the strings
    /// that only listen while their dampers are lifted (O(modes)).
    pub fn sounding_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active && v.energy() >= self.shortcuts.voice_silence).count()
    }

    /// Takes the live parameters ([`LiveParam`]) of `model`, as if each had
    /// been sent as an [`Event::Param`] (an engine built from an older
    /// model catches up with the edits made meanwhile).
    pub fn set_live(&mut self, model: &PianoModel) {
        for &p in LiveParam::ALL {
            p.set(&mut self.base, p.get(model));
        }
        self.refresh_live();
    }

    /// The live parameters from their base with the musician's controls
    /// applied.
    fn refresh_live(&mut self) {
        let resolved = self.base.with_macros();
        for &p in LiveParam::ALL {
            p.set(&mut self.live, p.get(&resolved));
        }
    }

    pub fn active_modes(&self) -> usize {
        let len = |b: &Option<BoardBank>| b.as_ref().map_or(0, |b| b.len());
        let board = if self.board_awake { len(&self.board) + len(&self.board_top) + self.board_low.as_ref().map_or(0, |b| b.len()) } else { 0 };
        let board_sym = if self.board_sym_awake { len(&self.board_sym) } else { 0 };
        self.voices.iter().filter(|v| v.active).map(|v| v.bank.len() + v.sym.len()).sum::<usize>() + board + board_sym
    }

    /// (note, bridge-force energy) of the active voices, for diagnostics.
    pub fn voice_energies(&self) -> Vec<(u8, f32)> {
        self.voices.iter().filter(|v| v.active).map(|v| (v.note, v.energy())).collect()
    }

    /// Adds this engine's state to `obs` (the interface's live view; no
    /// allocation, O(voices + observed modes)). Clear it first, then add
    /// every engine playing ([`crate::observe::Observation::merge`] for
    /// engines observed apart).
    pub fn observe(&self, obs: &mut crate::Observation) {
        let mut own = crate::Observation::default();
        for (k, &down) in own.keys.iter_mut().zip(&self.key_down) {
            k.down = down;
            // No voice: the strings rest, under their damper unless the
            // pedal lifts it.
            k.damper = if down { 0.0 } else { 1.0 - self.sustain as f32 };
        }
        for v in self.voices.iter().filter(|v| v.active) {
            let k = &mut own.keys[v.note as usize & 127];
            k.hammer |= v.contact.is_active();
            k.damper = k.damper.min(v.engagement as f32);
            // A sympathetic bank left alone below its threshold keeps its
            // last state, silent: not shown (else the undamped strings,
            // never freed, would glow once the views' scales come down).
            let sym = v.sym.energy();
            k.energy += v.bank.energy() + if sym < self.shortcuts.sym_silence { 0.0 } else { sym };
        }
        (own.sustain, own.soft, own.sostenuto) = (self.sustain as f32, self.soft as f32, self.sostenuto_down);
        // The low modes run apart when driven key by key: they come first.
        let mut n = self.board_low.as_ref().map_or(0, |b| b.amplitudes(&mut own.board));
        for b in self.board.iter().chain(&self.board_top) {
            n += b.amplitudes(&mut own.board[n..]);
        }
        // Asleep, the board keeps its last state, silent: at rest.
        if !self.board_awake {
            own.board[..n].fill(0.0);
        }
        own.board_modes = n;
        obs.merge(&own);
    }

    /// Bridge displacement fed to the strings at region `r` (diagnostics).
    pub fn bridge_input(&self, r: usize) -> &[f32] {
        &self.disp_in[r]
    }

    /// The primary bridge force (N: the struck strings and the knocks)
    /// summed at region `r` over the last sub-block rendered, and the
    /// bridge velocity there (m/s). Valid for a block of exactly
    /// [`SUB_BLOCK`] samples: the stage swap reads them block by block
    /// (piano-render's `bridge_signal` example).
    pub fn bridge_force(&self, r: usize) -> &[f32] {
        &self.region[r]
    }

    pub fn bridge_velocity(&self, r: usize) -> &[f32] {
        &self.disp[r]
    }

    /// Factor from the pressure at the microphones (Pa) to the output
    /// samples, the listening set-up's own `mic_gain_db` apart.
    pub fn pressure_scale(&self) -> f32 {
        self.gain * PRESSURE_GAIN
    }

    pub fn board_awake(&self) -> bool {
        self.board_awake
    }

    /// (note, primary energy, sympathetic energy) of the active voices.
    pub fn voice_energies_split(&self) -> Vec<(u8, f32, f32)> {
        self.voices.iter().filter(|v| v.active).map(|v| (v.note, v.bank.energy(), v.sym.energy())).collect()
    }

    fn sympathy(&self) -> bool {
        self.board.is_some() && self.inst.model.sympathetic.enabled
    }

    /// Mono render: mean of the stereo mix (overwrites `out`).
    pub fn process(&mut self, events: &[TimedEvent], out: &mut [f32]) {
        let mut st = std::mem::take(&mut self.stereo);
        {
            let [l, r] = &mut st;
            self.process_stereo(events, &mut l[..out.len()], &mut r[..out.len()]);
            for ((o, a), b) in out.iter_mut().zip(l.iter()).zip(r.iter()) {
                *o = 0.5 * (a + b);
            }
        }
        self.stereo = st;
    }

    /// Microphones of the listening set-up.
    pub fn mics(&self) -> usize {
        self.n_mics
    }

    /// Channels [`Self::process_channels`] writes: one per microphone when
    /// the set-up asks for it (`outputs_per_mic`), else 2 (the stereo mix).
    pub fn channels(&self) -> usize {
        if self.per_mic { self.n_mics } else { 2 }
    }

    /// Renders one block into `left`/`right` (overwritten): the stereo mix
    /// of the microphones, each by its gain and pan, whatever
    /// `outputs_per_mic` says. `events` must be sorted by frame and lie
    /// inside the block. Real-time safe: no allocation, no lock.
    pub fn process_stereo(&mut self, events: &[TimedEvent], left: &mut [f32], right: &mut [f32]) {
        let len = left.len();
        assert!(right.len() == len);
        let mut out = [left, right];
        self.process_into(events, &mut out, false);
    }

    /// Renders one block into `out` (overwritten, the same length): the
    /// set-up's channels ([`Self::channels`]), the others silent.
    /// Real-time safe.
    pub fn process_channels(&mut self, events: &[TimedEvent], out: &mut [&mut [f32]]) {
        let per_mic = self.per_mic;
        self.process_into(events, out, per_mic);
    }

    /// Whether the set-up asks for one output per microphone.
    pub fn per_mic(&self) -> bool {
        self.per_mic
    }

    /// Renders one block into `out` (overwritten, the same length): one
    /// channel per microphone with its gain when `per_mic` (as many as
    /// there are channels), else the stereo mix in the first two; the
    /// other channels silent. Real-time safe.
    pub fn process_into(&mut self, events: &[TimedEvent], out: &mut [&mut [f32]], per_mic: bool) {
        let len = out.first().map_or(0, |o| o.len());
        assert!(len <= self.cfg.max_block && out.iter().all(|o| o.len() == len));
        let _ftz = DenormalGuard::new();
        let mut ev = 0;
        let mut s0 = 0;
        while s0 < len {
            let s1 = (s0 + SUB_BLOCK).min(len);
            let n_ev = events[ev..].iter().take_while(|e| (e.frame as usize) < s1 || s1 == len).count();
            self.sub_block(&events[ev..ev + n_ev], s0, s1 - s0);
            let mics = &self.mic_buf[..self.n_mics];
            let written = if per_mic {
                for ((o, m), g) in out.iter_mut().zip(mics).zip(&self.own_gain) {
                    for (y, x) in o[s0..s1].iter_mut().zip(&m[..s1 - s0]) {
                        *y = g * x;
                    }
                }
                mics.len()
            } else if let [l, r, ..] = out {
                l[s0..s1].fill(0.0);
                r[s0..s1].fill(0.0);
                for (m, &(gl, gr)) in mics.iter().zip(&self.mix) {
                    for ((yl, yr), x) in l[s0..s1].iter_mut().zip(r[s0..s1].iter_mut()).zip(&m[..s1 - s0]) {
                        *yl += gl * x;
                        *yr += gr * x;
                    }
                }
                2
            } else {
                // One channel: the mix's mean.
                if let Some(o) = out.first_mut() {
                    o[s0..s1].fill(0.0);
                    for (m, &(gl, gr)) in mics.iter().zip(&self.mix) {
                        for (y, x) in o[s0..s1].iter_mut().zip(&m[..s1 - s0]) {
                            *y += 0.5 * (gl + gr) * x;
                        }
                    }
                }
                1
            };
            for o in out.iter_mut().skip(written) {
                o[s0..s1].fill(0.0);
            }
            ev += n_ev;
            s0 = s1;
        }
    }

    /// One sub-block (≤ SUB_BLOCK samples) into the microphone buffers;
    /// event frames are relative to the host block, which starts `offset`
    /// samples before this sub-block.
    fn sub_block(&mut self, events: &[TimedEvent], offset: usize, len: usize) {
        let mut mic_buf = std::mem::take(&mut self.mic_buf);
        let mut chans: [&mut [f32]; MICS] = mic_buf.each_mut().map(|v| &mut v[..len]);
        let nm = self.n_slots;
        let chans = &mut chans[..nm];
        let out_gain = self.smooth_gain(len);
        self.update_dampers(len);
        self.add_listeners();
        for r in self.region.iter_mut().chain(&mut self.region_sym).chain(&mut self.region_agraffe).chain(&mut self.basis).chain(&mut self.forced_mics) {
            r[..len].fill(0.0);
        }
        // Bridge displacement SUB_BLOCK samples ago, for the strings.
        let t0 = self.clock;
        if self.sympathy() {
            for (ring, inp) in self.disp_ring.iter().zip(self.disp_in.iter_mut()) {
                for (i, x) in inp[..len].iter_mut().enumerate() {
                    *x = ring[((t0 + i as u64).wrapping_sub(SUB_BLOCK as u64) as usize) & (RING - 1)];
                }
            }
        }
        // The body's halo renders up to each event, so that a strike takes
        // its slot in the bank from what the bank holds at its own frame,
        // however many come in one sub-block (crate::body).
        let body_gain = out_gain * PRESSURE_GAIN;
        for n in &mut self.noise_buf {
            n[..len].fill(0.0);
        }
        let mut pos = 0;
        for ev in events {
            let f = (ev.frame as usize).saturating_sub(offset).min(len);
            self.render(pos, f);
            self.render_body(pos, f, body_gain);
            pos = f;
            self.now = t0 + f as u64;
            self.handle(ev.event);
        }
        self.render(pos, len);
        self.render_body(pos, len, body_gain);
        // The rear lengths answer the bridge (the same delayed motion the
        // strings take) and push on the sympathetic board.
        if self.sympathy() {
            let silence = self.shortcuts.sym_silence;
            for ((bank, u), f) in self.rear.iter_mut().zip(&self.disp_in).zip(self.region_sym.iter_mut()) {
                if !bank.is_empty() && (bank.energy() >= silence || u[..len].iter().any(|&x| x != 0.0)) {
                    bank.process(Some(&u[..len]), &mut f[..len]);
                }
            }
        }

        self.render_knocks(len);
        for c in chans.iter_mut() {
            c.fill(0.0);
        }
        for d in &mut self.disp {
            d[..len].fill(0.0);
        }
        match self.board.as_mut() {
            None => {
                for r in &self.region {
                    for c in chans.iter_mut() {
                        for (y, x) in c.iter_mut().zip(&r[..len]) {
                            *y += out_gain * FORCE_GAIN * x;
                        }
                    }
                }
            }
            Some(board) => {
                let any = |b: &[Vec<f32>]| b.iter().any(|r| r[..len].iter().any(|&x| x != 0.0));
                let has_input = any(&self.region) || any(&self.basis);
                let thr = self.shortcuts.board_silence;
                for f in &mut self.rim_force {
                    f[..len].fill(0.0);
                }
                let has_frame = self.region_agraffe.iter().any(|r| r[..len].iter().any(|&x| x != 0.0));
                let run_frame = self.frame.is_some() && (has_frame || self.frame_awake);
                let has_sym = any(&self.region_sym);
                let run_sym = self.board_sym.is_some() && (has_sym || self.board_sym_awake);
                // (The modes that only radiate take the sympathetic forces
                // with the struck strings'.)
                let driven = has_input || (self.board_top.is_some() && has_sym);
                let run_board = driven || self.board_awake;
                if self.board_top.is_some() && run_board {
                    for ((a, x), y) in self.region_all.iter_mut().zip(&self.region).zip(&self.region_sym) {
                        for ((a, x), y) in a[..len].iter_mut().zip(&x[..len]).zip(&y[..len]) {
                            *a = x + y;
                        }
                    }
                }
                if let Some(pool) = self.pool.as_mut().filter(|_| run_board) {
                    // The parts of the board's banks (its high modes, its low
                    // modes, the sympathetic board, the modes that only
                    // radiate) and the frame in parallel,
                    // each into its own buffers, summed below in the same order
                    // as on one thread: the same result, bit for bit.
                    let with_rim = self.rim.is_some();
                    board.prepare(&self.region, len, true, with_rim);
                    if let Some(low) = self.board_low.as_mut() {
                        low.prepare(&self.basis, len, true, with_rim);
                    }
                    if let Some(bs) = self.board_sym.as_mut().filter(|_| run_sym) {
                        bs.prepare(&self.region_sym, len, false, false);
                    }
                    if let Some(top) = self.board_top.as_mut() {
                        top.prepare(&self.region_all, len, false, with_rim);
                    }
                    for b in self.side_out.iter_mut().flatten() {
                        b[..len].fill(0.0);
                    }
                    let (job, parts) = board.split();
                    let parts = Ptr(parts.as_mut_ptr());
                    let low = self.board_low.as_mut().map(|l| l.split()).map(|(job, parts)| (job, Ptr(parts.as_mut_ptr())));
                    let sym = self.board_sym.as_mut().filter(|_| run_sym).map(|b| b.split()).map(|(job, parts)| (job, Ptr(parts.as_mut_ptr())));
                    let top = self.board_top.as_mut().map(|b| b.split()).map(|(job, parts)| (job, Ptr(parts.as_mut_ptr())));
                    let frame = Ptr(self.frame.as_mut().map_or(std::ptr::null_mut(), |f| f as *mut BoardBank));
                    let side = Ptr(self.side_out.as_mut_ptr());
                    let region_agraffe = &self.region_agraffe;
                    // The top bank's jobs only when it runs (the frame's
                    // job comes right after the last bank's).
                    let banks = if top.is_some() { 4 } else { 3 };
                    pool.run(banks * PARTS + 1, &|i| {
                        let _ = (&parts, &low, &sym, &top, &frame, &side);
                        // SAFETY: every item touches its own part, bank and buffers.
                        unsafe {
                            match (i / PARTS, i % PARTS) {
                                (0, k) => (*parts.0.add(k)).run(&job),
                                (1, k) => {
                                    if let Some((job, parts)) = &low {
                                        (*parts.0.add(k)).run(job)
                                    }
                                }
                                (2, k) => {
                                    if let Some((job, parts)) = &sym {
                                        (*parts.0.add(k)).run(job)
                                    }
                                }
                                (3, k) if banks == 4 => {
                                    if let Some((job, parts)) = &top {
                                        (*parts.0.add(k)).run(job)
                                    }
                                }
                                _ if run_frame => {
                                    let mut out: [&mut [f32]; MICS] = (*side.0).each_mut().map(|v| &mut v[..len]);
                                    (*frame.0).process(region_agraffe, len, &mut out[..nm], None, None);
                                }
                                _ => {}
                            }
                        }
                    });
                    let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                    board.finish(chans, Some(&mut self.disp), rim_force);
                    if let Some(low) = self.board_low.as_mut() {
                        let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                        low.finish(chans, Some(&mut self.disp), rim_force);
                    }
                    if let Some(bs) = self.board_sym.as_mut().filter(|_| run_sym) {
                        let mut out: [&mut [f32]; MICS] = self.side_out[1].each_mut().map(|v| &mut v[..len]);
                        bs.finish(&mut out[..nm], None, None);
                    }
                    Self::board_feed(board, self.board_low.as_ref(), chans, &mut self.board_mics, &mut self.probes, len);
                    if let Some(top) = self.board_top.as_mut() {
                        // After the room's feed: what these modes radiate
                        // reaches the room as the sympathetic board's does.
                        let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                        top.finish(chans, None, rim_force);
                    }
                    self.board_awake = driven || board.energy() > thr || self.board_low.as_ref().is_some_and(|b| b.energy() > thr) || self.board_top.as_ref().is_some_and(|b| b.energy() > thr);
                    if let Some(rim) = self.rim.as_mut().filter(|_| self.board_awake || self.rim_awake) {
                        rim.process(&self.rim_force, len, chans);
                        self.rim_awake = self.board_awake || rim.energy() > self.shortcuts.board_silence;
                    }
                    for (k, run) in [run_frame, run_sym].into_iter().enumerate() {
                        if run {
                            for (c, s) in chans.iter_mut().zip(&self.side_out[k]) {
                                for (o, x) in c.iter_mut().zip(&s[..len]) {
                                    *o += x;
                                }
                            }
                        }
                    }
                } else {
                    if run_board {
                        let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                        board.process(&self.region, len, chans, Some(&mut self.disp), rim_force);
                        if let Some(low) = self.board_low.as_mut() {
                            let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                            low.process(&self.basis, len, chans, Some(&mut self.disp), rim_force);
                        }
                        Self::board_feed(board, self.board_low.as_ref(), chans, &mut self.board_mics, &mut self.probes, len);
                        if let Some(top) = self.board_top.as_mut() {
                            let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                            top.process(&self.region_all, len, chans, None, rim_force);
                        }
                        self.board_awake = driven || board.energy() > thr || self.board_low.as_ref().is_some_and(|b| b.energy() > thr) || self.board_top.as_ref().is_some_and(|b| b.energy() > thr);
                    } else {
                        for b in self.board_mics.iter_mut().chain(self.probes.iter_mut()) {
                            b[..len].fill(0.0);
                        }
                    }
                    if let Some(rim) = self.rim.as_mut().filter(|_| self.board_awake || self.rim_awake) {
                        rim.process(&self.rim_force, len, chans);
                        self.rim_awake = self.board_awake || rim.energy() > self.shortcuts.board_silence;
                    }
                    if let Some(frame) = self.frame.as_mut().filter(|_| run_frame) {
                        frame.process(&self.region_agraffe, len, chans, None, None);
                    }
                    if let Some(bs) = self.board_sym.as_mut().filter(|_| run_sym) {
                        bs.process(&self.region_sym, len, chans, None, None);
                    }
                }
                if let Some(frame) = self.frame.as_ref().filter(|_| run_frame) {
                    self.frame_awake = has_frame || frame.energy() > self.shortcuts.board_silence;
                }
                if let Some(bs) = self.board_sym.as_ref().filter(|_| run_sym) {
                    self.board_sym_awake = has_sym || bs.energy() > thr;
                }
                // The radiation's delay zones, each after its delay, into
                // their microphones (the board's own part as well, for the
                // room).
                if let Some(f) = self.fold_all.as_mut() {
                    f.fold(chans, len);
                }
                if let Some(f) = self.fold_board.as_mut() {
                    f.fold(&mut self.board_mics, len);
                }
                let chans = &mut chans[..self.n_mics];
                // The banks carry acceleration weights: the pressure is the
                // derivative of their sum (board::to_acceleration_weights).
                let fs = self.cfg.sample_rate as f32;
                for (ch, last) in chans.iter_mut().zip(self.mic_last.iter_mut()) {
                    for x in ch.iter_mut() {
                        let v = *x;
                        *x = (v - *last) * fs * out_gain * PRESSURE_GAIN;
                        *last = v;
                    }
                }
                for (b, last) in self.board_mics.iter_mut().chain(self.probes.iter_mut()).zip(self.board_last.iter_mut()) {
                    for x in b[..len].iter_mut() {
                        let v = *x;
                        *x = (v - *last) * fs * out_gain * PRESSURE_GAIN;
                        *last = v;
                    }
                }
                if self.live.noise.action_time != self.action_time {
                    self.action_time = self.live.noise.action_time;
                    self.action.set_time(self.action_time);
                }
                // The action's and the body's two sides, shared among the
                // mics by their pan.
                let [nl, nr] = &mut self.noise_buf;
                let (nl, nr) = (&mut nl[..len], &mut nr[..len]);
                // The body's halo is already in (render_body).
                self.action.render([&mut *nl, &mut *nr], out_gain * PRESSURE_GAIN);
                // Each microphone hears them from its distance: a far pair
                // later and weaker (crate::mics::noise_paths).
                let mask = self.noise_hist[0].len() - 1;
                let p0 = self.noise_pos;
                for (h, x) in self.noise_hist.iter_mut().zip([&*nl, &*nr]) {
                    for (t, v) in x.iter().enumerate() {
                        h[(p0 + t) & mask] = *v;
                    }
                }
                self.noise_pos = (p0 + len) & mask;
                let [hl, hr] = &self.noise_hist;
                for (m, (c, &(wl, wr))) in chans.iter_mut().zip(&self.noise_w).enumerate() {
                    let (g, d) = (self.noise_gain[m], self.noise_delay[m]);
                    if d == 0 && g == 1.0 {
                        for ((y, a), b) in c.iter_mut().zip(nl.iter()).zip(nr.iter()) {
                            *y += wl * a + wr * b;
                        }
                    } else {
                        for (t, y) in c.iter_mut().take(len).enumerate() {
                            let i = (p0 + t).wrapping_sub(d) & mask;
                            *y += g * (wl * hl[i] + wr * hr[i]);
                        }
                    }
                }
                if let Some(room) = self.room.as_mut() {
                    let feed = crate::room::BoardFeed {
                        mics: std::array::from_fn(|m| &self.board_mics[m][..len]),
                        probes: std::array::from_fn(|k| &self.probes[k][..len]),
                    };
                    room.process_with(chans, Some(feed));
                }
                // The strings' forced response, in pressure (not through the
                // differentiator: it starts as a sine does, with no step to
                // differentiate). After the room, which takes what the
                // microphones hold beyond the board's part as its source:
                // the correction is the direct sound's alone (the probes
                // keep their weights).
                for (c, f) in chans.iter_mut().zip(&self.forced_mics) {
                    for (y, x) in c.iter_mut().zip(&f[..len]) {
                        *y += x * out_gain * PRESSURE_GAIN;
                    }
                }
            }
        }
        // Bridge acceleration by backward difference of the velocity (its
        // half-sample lag is compensated in the strings' input phase).
        let fs = self.cfg.sample_rate as f32;
        for ((ring, d), last) in self.disp_ring.iter_mut().zip(&self.disp).zip(self.last_vel.iter_mut()) {
            for (i, &v) in d[..len].iter().enumerate() {
                ring[((t0 + i as u64) as usize) & (RING - 1)] = (v - *last) * fs;
                *last = v;
            }
        }
        self.clock += len as u64;

        let sympathy = self.sympathy();
        let drop = self.shortcuts.voice_silence * self.shortcuts.drop_partials;
        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            v.drop_silent_partials(drop);
            let listening = sympathy && (v.engagement < 0.5 || self.shortcuts.listen_all);
            if !listening && v.energy() < self.shortcuts.voice_silence {
                v.active = false;
            }
        }
        self.mic_buf = mic_buf;
    }

    fn render(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        let sympathy = self.sympathy();
        let with_frame = self.frame.is_some();
        let deaf = self.shortcuts.deaf_above;
        let t0 = self.clock;
        self.busy.clear();
        self.busy.extend(self.voices.iter().enumerate().filter(|(_, v)| v.active).map(|(i, _)| i));
        if let Some(pool) = self.pool.as_mut().filter(|_| self.busy.len() >= PARALLEL_MIN_VOICES) {
            // Every voice computes its forces in parallel, then they are summed
            // in voice order: the same result as one thread, bit for bit.
            let (table, disp_in, busy) = (&self.inst.table, &self.disp_in, &self.busy);
            let voices = Ptr(self.voices.as_mut_ptr());
            pool.run(busy.len(), &|i| {
                let _ = &voices;
                // SAFETY: `busy` holds distinct indices into `voices`.
                let v = unsafe { &mut *voices.0.add(busy[i]) };
                let nm = table.get(v.note);
                let bridge = (sympathy && v.engagement < deaf).then(|| (&disp_in[region_of(v.note)][from..to], SUB_BLOCK));
                v.compute(nm, with_frame, t0, from, to, bridge);
            });
            for &i in &self.busy {
                let v = &self.voices[i];
                let basis = Some(&mut self.basis[..]);
                v.mix(self.inst.table.get(v.note), &mut self.region, &mut self.region_sym, with_frame.then_some(&mut self.region_agraffe[..]), basis, from, to);
                v.mix_forced(self.inst.table.get(v.note), &mut self.forced_mics, self.n_mics, from, to);
            }
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let nm = self.inst.table.get(v.note);
            let r = region_of(v.note);
            // Fully damped strings do not resonate: skip their input.
            let bridge = (sympathy && v.engagement < deaf).then(|| (&self.disp_in[r][from..to], SUB_BLOCK));
            v.compute(nm, with_frame, t0, from, to, bridge);
            let basis = Some(&mut self.basis[..]);
            v.mix(nm, &mut self.region, &mut self.region_sym, with_frame.then_some(&mut self.region_agraffe[..]), basis, from, to);
            v.mix_forced(nm, &mut self.forced_mics, self.n_mics, from, to);
        }
    }

    /// The board's part of the mics (the mics hold only the board's banks so
    /// far) and its probes summed over the banks, for the room.
    fn board_feed(
        board: &BoardBank,
        low: Option<&BoardBank<{ crate::board::LOW_INPUTS }>>,
        mics: &[&mut [f32]],
        board_mics: &mut [Vec<f32>; MICS],
        probes: &mut [Vec<f32>; PROBES],
        len: usize,
    ) {
        for (b, m) in board_mics.iter_mut().zip(mics) {
            b[..len].copy_from_slice(&m[..len]);
        }
        let low = low.map(|l| l.probes());
        for (k, (p, b)) in probes.iter_mut().zip(board.probes()).enumerate() {
            p[..len].copy_from_slice(&b[..len]);
            if let Some(l) = &low {
                for (x, y) in p[..len].iter_mut().zip(l[k]) {
                    *x += y;
                }
            }
        }
    }

    /// Adds the pending frame knocks to the region forces.
    fn render_knocks(&mut self, len: usize) {
        for k in self.knocks.iter_mut().filter(|k| k.pos < k.len) {
            let r = &mut self.region[k.region];
            for x in r[..len].iter_mut() {
                if k.pos >= k.len {
                    break;
                }
                *x += k.peak * (std::f32::consts::PI * (k.pos as f32 + 0.5) / k.len as f32).sin();
                k.pos += 1;
            }
        }
    }

    fn knock(&mut self, region: usize, peak: f64) {
        if peak <= 0.0 || self.board.is_none() {
            return;
        }
        let len = ((self.live.noise.knock_time * self.cfg.sample_rate) as usize).max(2);
        if let Some(k) = self.knocks.iter_mut().find(|k| k.pos >= k.len) {
            *k = Knock { region, pos: 0, len, peak: peak as f32 };
        }
    }

    /// Moves every damper towards its target contact and sets the decay.
    fn update_dampers(&mut self, len: usize) {
        let law = self.live.damper;
        let dt = len as f64 / self.cfg.sample_rate;
        let pedal = law.pedal_engagement(self.sustain);
        let (k_fall, k_lift) = (1.0 - (-dt / law.fall_time).exp(), 1.0 - (-dt / law.lift_time).exp());
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let nm = self.inst.table.get(v.note);
            let target = if v.key_down || v.sostenuto || !nm.has_damper { 0.0 } else { pedal };
            let k = if target > v.engagement { k_fall } else { k_lift };
            let e = v.engagement + (target - v.engagement) * k;
            let e = if (e - target).abs() < 1e-3 { target } else { e };
            // The felt lands on the strings: momentum kick at the damper point.
            if v.engagement < 0.05 && e >= 0.05 {
                let noise = self.live.noise;
                let impulse = noise.damper_mass * noise.damper_speed * v.damper_speed;
                v.damper_kick(nm, impulse, law.position, law.width, noise.damper_landing);
            }
            v.engagement = e;
            v.set_damper_rate(nm.damper_rate(&self.live, v.engagement));
        }
        // The strings at rest soak up less of the board's vibration while
        // the pedal lifts their dampers (Soundboard::load_strings,
        // SoundboardParams::string_load_free).
        let free = self.inst.model.soundboard.map_or(0.0, |s| s.string_load_free.clamp(0.0, 1.0));
        let target = free + (1.0 - free) * pedal;
        let k = if target > self.board_load { k_fall } else { k_lift };
        let l = self.board_load + (target - self.board_load) * k;
        self.board_load = if (l - target).abs() < 1e-3 { target } else { l };
        let l = self.board_load;
        if (l - self.board_load_set).abs() >= 0.01 || (l == target && l != self.board_load_set) {
            self.board_load_set = l;
            for b in self.board.iter_mut().chain(&mut self.board_sym).chain(&mut self.board_top) {
                b.set_load(l);
            }
            if let Some(b) = self.board_low.as_mut() {
                b.set_load(l);
            }
        }
        // The same threshold as the listening strings' (add_listeners).
        let free = pedal < 0.5;
        if free != self.sym_free {
            self.sym_free = free;
            for v in self.voices.iter_mut().filter(|v| v.active) {
                v.set_sym_free(self.inst.table.get(v.note), free);
            }
        }
    }

    /// While the dampers are lifted (always, with [`Shortcuts::listen_all`]),
    /// give every damped key a listening string (all at once: setting one up
    /// is a copy, ≈ 1 µs).
    fn add_listeners(&mut self) {
        let law = self.live.damper;
        if !self.sympathy() || (law.pedal_engagement(self.sustain) >= 0.5 && !self.shortcuts.listen_all) {
            return;
        }
        let mut sounding = [false; 128];
        for v in self.voices.iter().filter(|v| v.active) {
            sounding[v.note as usize & 127] = true;
        }
        for note in 21..=law.highest_note.min(108) {
            if !sounding[note as usize] && !self.add_listener(note) {
                break;
            }
        }
    }

    /// Sets up a free voice as a listening string; false when none is free.
    fn add_listener(&mut self, note: u8) -> bool {
        let Some(v) = self.voices.iter_mut().find(|v| !v.active) else { return false };
        let nm = self.inst.table.get(note);
        v.setup(nm, true);
        v.set_sym_free(nm, self.sym_free);
        v.active = true;
        v.key_down = false;
        let law = self.live.damper;
        v.engagement = if nm.has_damper { law.pedal_engagement(self.sustain) } else { 0.0 };
        v.set_damper_rate(nm.damper_rate(&self.live, v.engagement));
        true
    }

    fn handle(&mut self, ev: Event) {
        match ev {
            // No key below A0 or above C8. Folded onto the end string, such a
            // note made a voice holding note 21 (or 108) that its note-off,
            // looking for its own number, never released: struck again and
            // again, the bottom string piled up undamped voices.
            Event::NoteOn { note, .. } | Event::NoteOff { note, .. } if !(21..=108).contains(&note) => {}
            Event::NoteOn { velocity: 0, note } => self.note_off(note),
            Event::NoteOn { note, velocity } => {
                let k = note as usize & 127;
                let window = (DUPLICATE_S * self.cfg.sample_rate) as u64;
                // The same key again while it is still down, a few ms later:
                // the same keystroke reaching the engine twice (a keyboard
                // connected through two MIDI paths). Struck again, the string
                // the first hammer had just set moving took a fresh blow: a
                // soft note came out harsh and metallic (2 ms apart: +21 dB
                // above 3 kHz).
                if self.key_down[k] && self.now.saturating_sub(self.key_down_at[k]) < window {
                    self.doubled = self.doubled.wrapping_add(1);
                    return;
                }
                self.key_down_at[k] = self.now;
                // Released and struck again while its hammer still presses
                // the string (a few ms): no action throws a hammer that has
                // not left the string. The key is down again (its damper
                // stays off), the strike is not played.
                if self.voices.iter().any(|v| v.active && v.note == note && v.contact.is_active()) {
                    self.key_down[k] = true;
                    self.key_up_at[k] = None;
                    for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
                        v.key_down = true;
                    }
                    return;
                }
                self.note_on(note, velocity)
            }
            Event::NoteOff { note, velocity } => {
                // Release velocity sets how fast the damper falls.
                let speed = if velocity == 0 { 1.0 } else { velocity as f64 / 64.0 };
                for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
                    v.damper_speed = speed;
                }
                self.note_off(note);
            }
            Event::Control { cc: 64, value } => {
                let new = value as f64 / 127.0;
                let delta = (new - self.sustain_prev).abs();
                // The damper rail knocks the frame near the middle of the
                // keyboard; on release the rail lowers the dampers, slower
                // than a key does (its travel is cushioned by felts).
                let peak = self.live.noise.pedal * delta;
                self.knock(region_of(54), 0.6 * peak);
                self.knock(region_of(66), 0.4 * peak);
                if new < self.sustain {
                    let speed = (delta * 0.3).clamp(0.02, 0.2);
                    for v in self.voices.iter_mut().filter(|v| v.active && !v.key_down) {
                        v.damper_speed = speed;
                    }
                }
                self.sustain = new;
                self.sustain_prev = new;
            }
            Event::Control { cc: 66, value } if self.live.mute_felt > 0.0 => self.mute_down = value >= 64,
            Event::Control { cc: 66, value } => {
                let down = value >= 64;
                if down && !self.sostenuto_down {
                    for v in self.voices.iter_mut().filter(|v| v.active) {
                        v.sostenuto = v.key_down;
                    }
                } else if !down {
                    for v in &mut self.voices {
                        v.sostenuto = false;
                    }
                }
                self.sostenuto_down = down;
            }
            Event::Control { cc: 67, value } => self.soft = value as f64 / 127.0,
            Event::Control { cc: 120 | 123, .. } | Event::AllNotesOff => {
                for v in &mut self.voices {
                    v.key_down = false;
                    v.sostenuto = false;
                }
                self.key_down = [false; 128];
                self.key_up_at = [None; 128];
            }
            Event::Control { .. } => {}
            Event::Param { id, value } => {
                id.set(&mut self.base, value as f64);
                self.refresh_live();
            }
        }
    }

    /// Output gain for a sub-block of `len` samples, moving towards the
    /// live target with a 20 ms time constant (no zipper on gain moves).
    fn smooth_gain(&mut self, len: usize) -> f32 {
        let target = db_to_gain(self.live.output_gain_db + OUTPUT_LEVEL_DB);
        let k = 1.0 - (-(len as f64) / (GAIN_SMOOTHING * self.cfg.sample_rate)).exp() as f32;
        self.gain += (target - self.gain) * k;
        if (self.gain - target).abs() < 1e-6 * target {
            self.gain = target;
        }
        self.gain
    }

    /// The body's halo over samples `a..b` of the sub-block, into the noise
    /// buffers, with the law as it stands (a live change applies from its
    /// own frame on).
    fn render_body(&mut self, a: usize, b: usize, gain: f32) {
        if self.board.is_none() || a >= b {
            return;
        }
        if self.live.body != *self.body.law() {
            let law = self.live.body;
            self.body.set_law(&law);
        }
        let [nl, nr] = &mut self.noise_buf;
        self.body.render([&mut nl[a..b], &mut nr[a..b]], gain);
    }

    fn note_on(&mut self, note: u8, velocity: u8) {
        let cfg = self.cfg;
        let repetition = self.repetition(note);
        // A key re-struck while its string still rings hits the moving string.
        let idx = self
            .voices
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| self.voices.iter().position(|v| !v.active))
            .unwrap_or_else(|| {
                // Steal the quietest voice.
                let mut best = 0;
                let mut e = f32::INFINITY;
                for (i, v) in self.voices.iter().enumerate() {
                    let ve = v.energy();
                    if ve < e {
                        e = ve;
                        best = i;
                    }
                }
                best
            });
        let nm = self.inst.table.get(note);
        let v = &mut self.voices[idx];
        if !(v.active && v.note == note) {
            v.setup(nm, false);
            v.set_sym_free(nm, self.sym_free);
        }
        // The key lifts its damper on the way down, before the hammer
        // arrives: a string still ringing under its damper is struck free
        // (it used to be struck damped, the damper lifting only afterwards).
        if v.engagement != 0.0 {
            v.engagement = 0.0;
            v.set_damper_rate(nm.damper_rate(&self.live, 0.0));
        }
        v.active = true;
        v.key_down = true;
        let softening = self.live.una_corda_softening;
        let vh = self.live.velocity.hammer_speed(velocity);
        // An upright's soft pedal shortens the throw (half-blow); a grand's
        // shifts the action (una corda).
        let thrown = vh * (1.0 - self.live.half_blow * self.soft).max(0.0) * repetition;
        let mute = if self.mute_down { self.live.mute_felt } else { 0.0 };
        // Overkill: twice the contact steps.
        let os = contact_oversampling(&cfg, note) * if cfg.quality == Quality::Overkill { 2 } else { 1 };
        v.strike(nm, thrown, os, self.soft * self.live.una_corda_shift, softening, mute);
        v.damper_speed = 1.0;
        // The key bottoms on the keybed as the hammer reaches the string.
        self.knock(region_of(note), self.live.noise.keybed * vh);
        if self.board.is_some() {
            // The action's knock renders from the start of the sub-block: it
            // waits for the note's own frame within it, then its delay. The
            // body's halo has rendered up to this frame (render_body).
            let within = self.now.saturating_sub(self.clock) as f64 / self.cfg.sample_rate;
            let tilt = crate::body::SpeedTilt::of(&self.live.body, note, vh);
            self.action.strike(note, self.live.noise.action * vh, within + self.live.noise.action_delay, tilt);
            // A live change of the body's law earlier in this sub-block
            // takes effect for this strike, not only at the next one.
            if self.live.body != *self.body.law() {
                let law = self.live.body;
                self.body.set_law(&law);
            }
            self.body.strike(note, thrown, 0.0);
        }
    }

    fn note_off(&mut self, note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
            v.key_down = false;
        }
        let k = note as usize & 127;
        if self.key_down[k] {
            self.key_down[k] = false;
            self.key_up_at[k] = Some(self.now);
        }
    }

    /// Hammer speed factor of a strike on `note` (also marks the key down):
    /// an upright's action re-strikes slower until `repetition_time` after
    /// the key's release, and a key struck again without a release (MIDI
    /// overlap) from the floor.
    fn repetition(&mut self, note: u8) -> f64 {
        let k = note as usize & 127;
        let since = if self.key_down[k] { Some(0) } else { self.key_up_at[k].map(|t| self.now - t) };
        self.key_down[k] = true;
        self.key_up_at[k] = None;
        let rt = self.live.repetition_time;
        match since {
            Some(dt) if rt > 0.0 => {
                let s = (dt as f64 / self.cfg.sample_rate / rt).min(1.0);
                let floor = self.live.repetition_floor.clamp(0.0, 1.0);
                floor + (1.0 - floor) * s * s * (3.0 - 2.0 * s)
            }
            _ => 1.0,
        }
    }
}

/// A note-on for a key already down within this time (s) is a duplicate of
/// the one before: no key repeats that fast (the hammer has not even left
/// the string).
const DUPLICATE_S: f64 = 0.03;

/// Sub-steps per sample during contact so that the shortest treble contacts
/// (≈ 0.4 ms) still span ~80 steps: 1 below C6 at 48 kHz, up to 4 at C8.
pub fn contact_oversampling(cfg: &EngineConfig, note: u8) -> usize {
    let per_ms = cfg.sample_rate / 1000.0;
    let need = match note {
        0..=83 => 40.0,
        84..=95 => 80.0,
        _ => 200.0,
    };
    ((need / per_ms).ceil() as usize).clamp(1, 8)
}

/// MIDI velocity → hammer velocity (see [`crate::params::VelocityLaw`]).
pub fn hammer_velocity(cfg: &EngineConfig, velocity: u8) -> f64 {
    cfg.model.velocity.hammer_speed(velocity)
}

/// Contact duration (s) and peak force (N) of one strike on a string at
/// rest. Allocates: analysis use only.
pub fn strike_stats(cfg: &EngineConfig, note: u8, velocity: u8, oversample: Option<usize>) -> (f64, f64) {
    let (t, f, _) = strike_stats_full(cfg, note, velocity, oversample);
    (t, f)
}

/// Contact duration (s), peak force (N) and largest felt compression (m).
pub fn strike_stats_full(cfg: &EngineConfig, note: u8, velocity: u8, oversample: Option<usize>) -> (f64, f64, f64) {
    let board = cfg.model.board();
    let nm = crate::note_model::NoteModel::build(&cfg.model, board.as_ref(), None, note, cfg.sample_rate, MAX_MODES);
    let mut v = Voice::new(cfg.sample_rate, cfg.max_block);
    v.setup(&nm, false);
    let os = oversample.unwrap_or_else(|| contact_oversampling(cfg, note));
    v.strike(&nm, hammer_velocity(cfg, velocity), os, 0.0, 0.0, 0.0);
    let mut out = vec![0.0f32; 256];
    while v.contact.is_active() {
        v.contact.run(&mut out, None);
    }
    (v.contact.contact_time(), v.contact.max_force(), v.contact.max_compression())
}

/// Contact force (N) at each output sample of one strike on a string at
/// rest, until the episode ends. Allocates: analysis use only.
pub fn strike_force(cfg: &EngineConfig, note: u8, velocity: u8) -> Vec<f64> {
    strike_forces(cfg, note, velocity).iter().map(|f| f.iter().sum()).collect()
}

/// The same, string by string.
pub fn strike_forces(cfg: &EngineConfig, note: u8, velocity: u8) -> Vec<[f64; crate::hammer::NS]> {
    let board = cfg.model.board();
    let nm = crate::note_model::NoteModel::build(&cfg.model, board.as_ref(), None, note, cfg.sample_rate, MAX_MODES);
    let mut v = Voice::new(cfg.sample_rate, cfg.max_block);
    v.setup(&nm, false);
    v.strike(&nm, hammer_velocity(cfg, velocity), contact_oversampling(cfg, note), 0.0, 0.0, 0.0);
    let mut out = [0.0f32; 1];
    let mut force = Vec::new();
    while v.contact.is_active() && v.contact.run(&mut out, None) == 1 {
        force.push(v.contact.string_forces());
    }
    force
}

fn db_to_gain(db: f64) -> f32 {
    10f32.powf(db as f32 / 20.0)
}
