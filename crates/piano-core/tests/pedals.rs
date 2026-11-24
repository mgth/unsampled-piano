//! Behaviour of dampers, pedals and sympathetic resonance.

use piano_core::instrument::Instrument;
use piano_core::{Engine, EngineConfig, Event, LiveParam, TimedEvent};
use std::sync::{Arc, OnceLock};

fn instrument() -> Arc<Instrument> {
    static I: OnceLock<Arc<Instrument>> = OnceLock::new();
    I.get_or_init(|| Arc::new(Instrument::new(EngineConfig::default().model, 48_000.0))).clone()
}

/// Renders a script of (time s, event) for `dur` seconds; returns the mono
/// output and the engine.
fn render(script: &[(f64, Event)], dur: f64) -> (Vec<f32>, Engine) {
    render_on(instrument(), script, dur)
}

/// [`render`] on a given instrument.
fn render_on(inst: Arc<Instrument>, script: &[(f64, Event)], dur: f64) -> (Vec<f32>, Engine) {
    let cfg = EngineConfig::default();
    let mut e = Engine::with_instrument(cfg, inst);
    let bs = cfg.max_block;
    let n = (dur * cfg.sample_rate) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let ev: Vec<TimedEvent> = script
            .iter()
            .filter_map(|&(t, ev)| {
                let f = (t * cfg.sample_rate) as usize;
                (f / bs == b).then_some(TimedEvent { frame: (f % bs) as u32, event: ev })
            })
            .collect();
        e.process(&ev, blk);
    }
    (out, e)
}

fn rms_db(x: &[f32]) -> f64 {
    let p = x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64;
    10.0 * p.max(1e-30).log10()
}

/// Bridge-force energy (dB) of one key's strings at the end of a script.
fn key_energy_db(script: &[(f64, Event)], dur: f64, note: u8) -> f64 {
    let (_, e) = render(script, dur);
    let en = e.voice_energies().iter().find(|v| v.0 == note).map_or(0.0, |v| v.1);
    10.0 * (en as f64).max(1e-30).log10()
}

/// Initial T60 of a key's strings after its release at `t`, from its energy
/// (the slower tail of weakly damped partials is not measured).
fn string_t60(script: &[(f64, Event)], t: f64, note: u8) -> f64 {
    let (a, b) = (key_energy_db(script, t + 0.01, note), key_energy_db(script, t + 0.06, note));
    60.0 * 0.05 / (a - b).max(1e-6)
}

fn on(note: u8, velocity: u8) -> Event {
    Event::NoteOn { note, velocity }
}

fn off(note: u8) -> Event {
    Event::NoteOff { note, velocity: 64 }
}

fn cc(cc: u8, value: u8) -> Event {
    Event::Control { cc, value }
}

#[test]
fn dampers_stop_mid_notes_faster_than_bass_and_skip_the_top() {
    let t60 = |note: u8| string_t60(&[(0.0, on(note, 100)), (0.5, off(note))], 0.5, note);
    let (bass, mid, top) = (t60(28), t60(60), t60(100));
    eprintln!("T60 after release: E1 {bass:.2} s, C4 {mid:.2} s, E7 (no damper) {top:.2} s");
    assert!(mid < 0.4, "C4 damped too slowly: {mid}");
    assert!(bass > mid, "bass dampers should be less effective");
    assert!(top > 2.0 * mid, "keys above the dampers must keep ringing");
}

#[test]
fn half_pedal_damps_partially() {
    let t60 = |pedal: u8| string_t60(&[(0.0, cc(64, pedal)), (0.01, on(60, 100)), (0.5, off(60))], 0.5, 60);
    let (up, half, down) = (t60(0), t60(50), t60(127));
    eprintln!("C4 T60 after release: pedal up {up:.2} s, half {half:.2} s, down {down:.2} s");
    assert!(up < half && half < down, "{up} {half} {down}");
}

#[test]
fn sostenuto_holds_only_the_keys_down_when_pressed() {
    let script = [
        (0.0, on(48, 100)),
        (0.1, cc(66, 127)),
        (0.2, off(48)),
        (0.25, on(60, 100)),
        (0.35, off(60)),
    ];
    let (_, e) = render(&script, 1.2);
    let energy = |n: u8| e.voice_energies().iter().find(|v| v.0 == n).map_or(0.0, |v| v.1);
    let (held, other) = (energy(48), energy(60));
    eprintln!("at 1.2 s: C3 (sostenuto) {held:e}, C4 (played after) {other:e}");
    // 39 dB apart since the main strings slant (docs/bridges.md): C3's
    // drive point moved towards the spine, 44 dB before.
    assert!(held > 3e3 * other.max(1e-30), "sostenuto did not hold C3");
}

/// Una corda: the shifted hammer meets softer felt (darker attack) and
/// strikes 2 of 3 strings, which favours the slow out-of-phase modes (longer
/// aftersound, Weinreich). The model does not reproduce the level drop of a
/// real grand (see docs/phase5.md). Since the bridge spreading and the board
/// taper (docs/sustain.md) weakened the unison coupling, the aftersound gain is
/// note-dependent (−7 to +7 dB) and about nil on average: the test only
/// checks that the sustain is not lost, over several keys.
#[test]
fn una_corda_is_darker_and_keeps_its_sustain() {
    let measure = |soft: u8, note: u8| {
        let (x, _) = render(&[(0.0, cc(67, soft)), (0.01, on(note, 100))], 1.0);
        let sr = 48_000.0;
        let seg = |a: f64, b: f64| &x[(a * sr) as usize..(b * sr) as usize];
        // Brightness proxy: energy of the first difference over the energy.
        let att = seg(0.01, 0.06);
        let d: f64 = att.windows(2).map(|w| ((w[1] - w[0]) as f64).powi(2)).sum();
        let e: f64 = att.iter().map(|v| (*v as f64).powi(2)).sum();
        (10.0 * (d / e).log10(), rms_db(seg(0.6, 1.0)) - rms_db(seg(0.01, 0.1)))
    };
    let ((b0, _), (b1, _)) = (measure(0, 60), measure(127, 60));
    eprintln!("C4 brightness {b0:.1} → {b1:.1} dB");
    assert!(b1 < b0 - 0.3, "una corda should be darker");
    let notes = [48u8, 55, 60, 64, 67, 72, 79];
    let gain: f64 = notes.iter().map(|&n| measure(127, n).1 - measure(0, n).1).sum::<f64>() / notes.len() as f64;
    eprintln!("mean late/early change with una corda: {gain:+.1} dB");
    assert!(gain > -1.5, "una corda should not lose sustain");
}

/// An upright's pedals: the soft pedal shortens the hammers' throw instead
/// of shifting the action, and the middle pedal lowers a practice mute.
#[test]
fn upright_soft_pedal_and_practice_mute() {
    let upright = [
        (0.0, Event::Param { id: LiveParam::UnaCordaShift, value: 0.0 }),
        (0.0, Event::Param { id: LiveParam::HalfBlow, value: 0.1 }),
        (0.0, Event::Param { id: LiveParam::MuteFelt, value: 3.0 }),
    ];
    let measure = |pedal: Option<u8>| {
        let mut script = upright.to_vec();
        script.extend(pedal.map(|c| (0.0, cc(c, 127))));
        script.push((0.01, on(60, 100)));
        let (x, _) = render(&script, 0.3);
        let att = &x[480..2880];
        let d: f64 = att.windows(2).map(|w| ((w[1] - w[0]) as f64).powi(2)).sum();
        let e: f64 = att.iter().map(|v| (*v as f64).powi(2)).sum();
        (rms_db(att), 10.0 * (d / e).log10())
    };
    let ((l0, b0), (l1, b1), (l2, b2)) = (measure(None), measure(Some(67)), measure(Some(66)));
    eprintln!("C4: soft pedal {:+.1} dB, brightness {:+.1} dB; mute {:+.1} dB, brightness {:+.1} dB", l1 - l0, b1 - b0, l2 - l0, b2 - b0);
    assert!((-3.0..-0.3).contains(&(l1 - l0)), "half blow: {:.1} dB", l1 - l0);
    assert!(b1 - b0 > -1.5, "half blow should barely darken: {:.1} dB", b1 - b0);
    // The graded felt (docs/felt.md) brightens less with the hammer's speed
    // than the two-layer hammer did (brighter at mf, as bright at ff), so
    // the muted C4 loses 2.6 dB of the proxy where it lost 3.3. With the
    // middle's stiffer shank (docs/felt.md) it loses 0.9 dB of level where
    // it lost 1.2.
    assert!(b2 < b0 - 2.0 && l2 < l0 - 0.7, "the mute should be duller and quieter");
    // With a mute, the middle pedal no longer holds the keys down.
    let middle = |script: &[(f64, Event)]| {
        let mut s = script.to_vec();
        s.extend([(0.0, on(48, 100)), (0.1, cc(66, 127)), (0.2, off(48))]);
        key_energy_db(&s, 1.0, 48)
    };
    let (sostenuto, mute) = (middle(&[]), middle(&upright));
    eprintln!("C3 0.8 s after release, middle pedal down: sostenuto {sostenuto:.0} dB, mute {mute:.0} dB");
    // Held, it would be level with the sostenuto. Damped, it lay 38 dB
    // under it; 28 since the sideways coupling of pol_B (h 0.6 at C3, which
    // the dampers grip less: docs/fundamental_sustain.md); 19 since the
    // main strings slant (C3's drive point nearer the spine, where the
    // board takes less of its damped string's energy: docs/bridges.md).
    assert!(mute < sostenuto - 15.0, "C3 held by the mute's pedal");
}

/// Without the grand's double escapement, an upright re-strikes a key weaker
/// until its action has reset after the release; later, as a grand.
#[test]
fn upright_repeats_slower_than_a_grand() {
    let level = |rep: f32, gap: f64| {
        let script = [
            (0.0, Event::Param { id: LiveParam::RepetitionTime, value: rep }),
            (0.0, on(60, 100)),
            (0.1, off(60)),
            (0.1 + gap, on(60, 100)),
        ];
        let (x, _) = render(&script, 0.2 + gap);
        let t = ((0.1 + gap) * 48_000.0) as usize;
        rms_db(&x[t..t + 2400])
    };
    let (soon, late) = (level(0.06, 0.01) - level(0.0, 0.01), level(0.06, 0.1) - level(0.0, 0.1));
    eprintln!("re-strike 10 ms after the release: {soon:+.1} dB; 100 ms after: {late:+.2} dB");
    // The 50 ms level also depends on the board under C4: −3.3 dB with the
    // rectangle, −2.6 with the grand's outline (the default since 2026-10-28).
    assert!(soon < -2.0, "an upright should re-strike weaker at once");
    assert!(late.abs() < 0.05, "and as a grand once its action has reset");
}

#[test]
fn pedal_lets_the_octave_resonate_more_than_the_semitone() {
    // Pedal down, C3 struck briefly; listen to the free C4 and C#4 strings.
    let script = [(0.0, cc(64, 127)), (0.3, on(48, 110)), (0.45, off(48))];
    let (_, e) = render(&script, 1.5);
    let energy = |n: u8| e.voice_energies().iter().find(|v| v.0 == n).map_or(0.0, |v| v.1);
    let (octave, twelfth, semitone) = (energy(60), energy(67), energy(61));
    eprintln!("sympathetic energy: C4 {octave:e}, G4 {twelfth:e}, C#4 {semitone:e}");
    assert!(octave > 30.0 * semitone, "octave {octave:e} semitone {semitone:e}");
    assert!(twelfth > 10.0 * semitone, "twelfth {twelfth:e} semitone {semitone:e}");
}

/// A key re-struck while its released string still rings under the damper
/// is struck free, as a fresh one: the key lifts the damper on its way down,
/// before the hammer arrives.
#[test]
fn a_restruck_key_is_struck_with_its_damper_lifted() {
    let on = |t| (t, Event::NoteOn { note: 60, velocity: 100 });
    let off = |t| (t, Event::NoteOff { note: 60, velocity: 64 });
    // Released at 0.3 s; the damped string still rings at 0.6 s.
    let (again, _) = render(&[on(0.0), off(0.3), on(0.6)], 0.7);
    let (fresh, _) = render(&[on(0.6)], 0.7);
    let peak = |x: &[f32]| x[(0.6 * 48_000.0) as usize..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let db = 20.0 * (peak(&again) / peak(&fresh)).log10();
    assert!(db.abs() < 1.0, "re-struck attack {db:.1} dB from a fresh one");
}

/// Once a note has died away, the undamped strings that heard it are left
/// alone and the board sleeps: what the engine shows of them is rest, not
/// the state they were left in (the live view would light it up).
#[test]
fn observed_rest_after_a_note() {
    let (_, e) = render(&[(0.01, on(60, 127)), (0.5, off(60))], 10.0);
    assert!(e.active_voices() > 0, "the undamped strings keep their listening voices");
    let mut obs = piano_core::Observation::default();
    e.observe(&mut obs);
    let lit: Vec<(usize, f32)> = obs.keys.iter().enumerate().filter(|(_, k)| k.energy > 0.0).map(|(m, k)| (m, k.energy)).collect();
    assert!(lit.is_empty(), "{lit:?}");
    assert!(!e.board_awake());
    assert!(obs.board[..obs.board_modes].iter().all(|&a| a == 0.0));
}

/// What is left of a signal at 48 kHz above 5 kHz (eighth-order Butterworth
/// high-pass, four biquads).
fn above_5k(x: &[f32]) -> Vec<f64> {
    let mut y: Vec<f64> = x.iter().map(|v| *v as f64).collect();
    let (sn, cs) = (std::f64::consts::TAU * 5000.0 / 48_000.0).sin_cos();
    for q in [0.5098, 0.6013, 0.9000, 2.5629] {
        let alpha = sn / (2.0 * q);
        let (b0, b1, a1, a2) = ((1.0 + cs) / 2.0 / (1.0 + alpha), -(1.0 + cs) / (1.0 + alpha), -2.0 * cs / (1.0 + alpha), (1.0 - alpha) / (1.0 + alpha));
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        for v in y.iter_mut() {
            let out = b0 * (*v + x2) + b1 * x1 - a1 * y1 - a2 * y2;
            (x2, x1, y2, y1) = (x1, *v, y1, out);
            *v = out;
        }
    }
    y
}

/// A bass key repeated under the pedal: each strike finds its strings
/// ringing, and their stretching by the partials already there holds
/// through the hammer's contact. Cut at the strike and started again from
/// nothing after it, the tension stepped and every longitudinal mode rang:
/// a tick above 5 kHz at each repeat, 20 to 40 dB over a first strike's.
#[test]
fn a_bass_key_repeated_under_the_pedal_does_not_tick() {
    const STRIKES: usize = 8;
    let at = |i: usize| 0.05 + 0.2 * i as f64;
    let mut script = vec![(0.0, cc(64, 127))];
    for i in 0..STRIKES {
        script.extend([(at(i), on(33, 80)), (at(i) + 0.1, off(33))]);
    }
    // The longitudinal modes at the Concert grand's level, without and
    // with the strike's own stretching (the tick came at the end of the
    // contact, or at the strike itself).
    let mut plain = EngineConfig::default().model;
    plain.nonlinear.coupling = 1.0;
    let mut stretch = plain;
    stretch.nonlinear.strike_stretch = true;
    stretch.nonlinear.strike_scale = 0.25;
    for (name, model) in [("partials alone", plain), ("strike_stretch", stretch)] {
        let inst = Arc::new(Instrument::new(model, 48_000.0));
        let (x, _) = render_on(inst.clone(), &script, at(STRIKES));
        let highs = above_5k(&x);
        // The loudest 2 ms of the 20 ms after each strike, over the 20 ms
        // before it: what the strike itself adds up there.
        let db = |p: f64| 10.0 * p.max(1e-30).log10();
        let power = |c: &[f64]| c.iter().map(|v| v * v).sum::<f64>() / c.len() as f64;
        let jump = |i: usize| {
            let from = (at(i) * 48_000.0) as usize;
            db(highs[from..from + 960].chunks(96).map(power).fold(0.0, f64::max)) - db(power(&highs[from - 960..from]))
        };
        let mut jumps: Vec<f64> = (2..STRIKES).map(jump).collect();
        jumps.sort_by(f64::total_cmp);
        let rise = (jumps[(jumps.len() - 1) / 2] + jumps[jumps.len() / 2]) / 2.0;
        eprintln!("{name}: above 5 kHz, a repeat's loudest 2 ms {rise:+.1} dB over the 20 ms before it (median; {:+.1} to {:+.1})", jumps[0], jumps[jumps.len() - 1]);
        // (+9.1 dB as it should be, the new attack's own highs; +27.8 dB
        // with the tick.)
        assert!(rise < 19.0, "{name}: a repeated A1 jumps {rise:.1} dB above 5 kHz at the strike");
        // And the repeats are struck: a key that stayed silent under the
        // pedal would not tick either. What they add to a render where the
        // first strike alone rings on, over that render, in the 100 ms after
        // each of them.
        let alone: Vec<(f64, Event)> = script.iter().copied().filter(|&(t, ev)| t <= at(0) || !matches!(ev, Event::NoteOn { .. })).collect();
        let (first, _) = render_on(inst, &alone, at(STRIKES));
        let added: Vec<f32> = x.iter().zip(&first).map(|(a, b)| a - b).collect();
        let attack = |i: usize| {
            let from = (at(i) * 48_000.0) as usize;
            rms_db(&added[from..from + 4800]) - rms_db(&first[from..from + 4800])
        };
        let least = (1..STRIKES).map(attack).fold(f64::INFINITY, f64::min);
        eprintln!("{name}: the repeats add {least:+.1} dB or more over the first strike left to ring");
        // (-0.2 dB at the first repeat, more at each one after; the two
        // renders are the same when the repeats are lost.)
        assert!(least > -6.0, "{name}: a repeat of A1 adds {least:.1} dB over the first strike left to ring");
    }
}

/// The pedals as the engine shows them after a script.
fn observed_pedals(script: &[(f64, Event)]) -> (f32, bool, f32) {
    let (_, e) = render(script, 0.05);
    let mut obs = piano_core::Observation::default();
    e.observe(&mut obs);
    (obs.sustain, obs.middle, obs.soft)
}

/// A pedal held from the interface stays down whatever its controller
/// says; let go, the controller's own position applies again.
#[test]
fn a_held_pedal_stays_down_ahead_of_its_controller() {
    let hold = |c: u8, down: bool| Event::Hold { cc: c, down };
    // Held, then the controller pressed and let go: still down.
    let held = [(0.0, hold(64, true)), (0.01, cc(64, 127)), (0.02, cc(64, 0))];
    assert_eq!(observed_pedals(&held).0, 1.0);
    // Let go of the hold: the controller's own (up) position.
    let mut released = held.to_vec();
    released.push((0.03, hold(64, false)));
    assert_eq!(observed_pedals(&released).0, 0.0);
    // The controller half down under the hold, the hold let go: half down.
    let half = [(0.0, cc(64, 64)), (0.01, hold(64, true)), (0.02, hold(64, false))];
    assert!((observed_pedals(&half).0 - 64.0 / 127.0).abs() < 1e-6);
    // The soft and middle pedals alike.
    let others = [(0.0, hold(67, true)), (0.0, hold(66, true)), (0.01, cc(67, 0)), (0.01, cc(66, 0))];
    let (_, middle, soft) = observed_pedals(&others);
    assert!(middle && soft == 1.0);
}

/// The middle pedal shows down while it works a practice mute as it does
/// for the sostenuto.
#[test]
fn the_middle_pedal_shows_down_for_the_mute_too() {
    let mute = (0.0, Event::Param { id: LiveParam::MuteFelt, value: 3.0 });
    assert!(observed_pedals(&[mute, (0.01, cc(66, 127))]).1);
    assert!(!observed_pedals(&[mute, (0.01, cc(66, 127)), (0.02, cc(66, 0))]).1);
    assert!(observed_pedals(&[(0.01, cc(66, 127))]).1);
}
