//! Contextual help of every parameter: what it changes on the piano, in
//! plain words, and how the model implements it. Arrays share one entry
//! ("room.size" covers "room.size[1]"); a test checks that every parameter
//! has one.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Help {
    /// The physical part and what it does to the sound.
    pub what: &'static str,
    /// The model's equations and where the default comes from.
    pub how: &'static str,
}

/// Help of a parameter path (array indices ignored).
pub fn help(path: &str) -> Option<Help> {
    let key = strip_indices(path);
    HELP.iter().find(|h| h.0 == key).map(|&(_, what, how)| Help { what, how })
}

/// "soundboard.mics[1][2]" → "soundboard.mics".
fn strip_indices(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut depth = 0;
    for c in path.chars() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// (path, what, how)
const HELP: &[(&str, &str, &str)] = &[
    // General
    (
        "a4",
        "Pitch of the A above middle C, which the whole tuning follows. 440 Hz is the modern \
         standard; some orchestras tune to 442, baroque ensembles to 415.",
        "Each key's fundamental is a4·2^((n − 69)/12), shifted by the stretch law. The string \
         masses follow from it (μ = T/(2·L·f0)²), so the instrument is rebuilt.",
    ),
    (
        "limiter",
        "Keeps the output from clipping: loud attacks, the treble's above all, can pass full scale long \
         before the music sounds loud, and clip further down the chain (a harsh crack). On by default. \
         The status bar shows CLIP when samples go over, and how much the limiter takes off.",
        "Without latency: the gain falls at once to what a sample above the ceiling needs and comes \
         back with a 100 ms time constant, both channels alike (docs/limiter.md).",
    ),
    ("limiter_ceiling_db", "The highest level the limiter lets out.", "Ceiling in dBFS (−1 by default, at most 0)."),
    (
        "output_gain_db",
        "Overall level, like the gain of the microphone preamp. It does not change the tone.",
        "Applied to both channels after the room, smoothed with a 20 ms time constant so that \
         automation does not click. Presets use it to level close and distant microphone set-ups.",
    ),
    (
        "una_corda_softening",
        "How much softer the tone gets with the left (una corda) pedal. The pedal shifts the whole \
         action sideways: each hammer strikes one string fewer, with fresh felt that the strings \
         have not grooved and hardened.",
        "With the soft pedal at s (CC 67, 0–1), the first string of each bichord and trichord gets \
         (1 − s) of the hammer force, the felt stiffness Q0 is multiplied by 1 − softening·s, and \
         the top felt is deeper by half that (+30 % at 0.6): the hard core is reached later. Read \
         at each strike.",
    ),
    (
        "una_corda_shift",
        "What the left pedal does. On a grand (1) it shifts the action so that each hammer \
         strikes one string fewer; on an upright (0) it does not shift anything, it moves the \
         hammers closer to the strings instead (see half blow).",
        "The first string of each bichord and trichord gets (1 − shift·s) of the hammer force, and \
         the felt softening is applied with shift·s, for the soft pedal at s (CC 67, 0–1).",
    ),
    (
        "half_blow",
        "How much quieter the left pedal of an upright plays: it moves the hammers closer to the \
         strings, so they travel less and strike more slowly. 0 on a grand.",
        "Hammer speed × (1 − half_blow·s), soft pedal at s. 0.1 matches a commercial modelled \
         piano's upright (−1 dB, timbre nearly unchanged).",
    ),
    (
        "mute_felt",
        "The middle pedal of many uprights lowers a strip of felt between hammers and strings, \
         for practising quietly: a muffled, felty tone. This is the strip's thickness; 0 makes \
         the middle pedal a sostenuto, as on a grand.",
        "While CC 66 is down, new strikes go through a strip of t mm in series with the felt: Q0 × \
         min(1, 0.09/t), hysteresis × 3, core reached t mm later. An estimate (no reference).",
    ),
    (
        "repetition_time",
        "An upright repeats a note more slowly than a grand: it lacks the grand's double \
         escapement, so the key must come back almost all the way up before the hammer can be \
         thrown again. A note played again sooner than this after its key's release comes out \
         weaker. 0: a grand, no limit.",
        "Hammer speed × (floor + (1 − floor)·smoothstep(Δt/T)), Δt from the note-off to the \
         next note-on of the same key (0 when there was no note-off). Uprights repeat up to \
         ≈ 7 notes/s against ≈ 14 for a grand; 0.06 s is an estimate.",
    ),
    (
        "repetition_floor",
        "How strong a note played again at once is, on an upright: never silent, only weaker.",
        "Share of the hammer speed at Δt = 0 (0.5: about −8 dB).",
    ),
    // Velocity
    (
        "velocity.vmin",
        "Speed of the hammer for the softest key press (MIDI velocity 1). Lower: a quieter, \
         duller pianissimo.",
        "Hammer speed = vmin·(vmax/vmin)^(t^curve), t = (velocity − 1)/126. Read at each note-on.",
    ),
    (
        "velocity.vmax",
        "Speed of the hammer for the hardest key press (velocity 127). Higher: a louder and much \
         brighter fortissimo, since a fast hammer squeezes its felt harder.",
        "Upper end of the velocity map (see the curve). The defaults, 0.26–4.4 m/s with curve \
         0.75, follow two Disklavier grands within 1–2.5 dB (tools/velocity_fit.py), and keep \
         velocity 100 at 2.8 m/s, where the felt was set on the brightness of attack and \
         sustain (docs/brightness.md).",
    ),
    (
        "velocity.curve",
        "Shape of the velocity response: lower spreads the soft end (pianissimo quieter, \
         more room between pp and p), 1 spreads it evenly.",
        "Exponent on t in the velocity map: 1 is the exponential, logarithmic map of recording \
         pianos (Goebl & Bresin 2003); 0.75 fits two Disklavier grands (BiVib, Liang), whose \
         level falls faster at the soft end.",
    ),
    // Hammer
    (
        "hardness",
        "Voicing: how hard the hammer felt is. Harder felt gives a brighter, more percussive tone, \
         softer felt a rounder, mellower one. Technicians voice a piano by needling or hardening \
         the felt.",
        "Multiplies the felt stiffness Q0 of every key in F = Q0·u^p (u: felt compression in mm), \
         from Stulov's measured hammers (ICA 2009). The default 0.3 makes the top felt softer than \
         his; the hard core underneath gives the bite at forte (docs/middle_highs.md).",
    ),
    (
        "hardness_slope",
        "How quickly the hammers get harder from middle C to the top. Treble hammers are smaller \
         and harder; more slope brightens the upper register.",
        "Q0 ×= e^(slope·(key − 40)) above C4 (key 40, counting A0 as 1), on top of the growth of \
         Stulov's fit, e^(0.045·key).",
    ),
    (
        "hardness_slope_bass",
        "The same, from middle C down to the lowest key. Positive values make the bass hammers \
         softer than the middle ones, negative values harder.",
        "Q0 ×= e^(slope·(key − 40)) below C4.",
    ),
    (
        "hardness_slope_top",
        "Extra hardness of the top octave and a half's hammers, which voicers harden so that the \
         highest notes ring out.",
        "Q0 ×= e^(slope·(key − 64)) above C6, on top of hardness_slope (docs/top_level.md).",
    ),
    (
        "core_hardness",
        "Hammer felt is denser inside than outside. Played softly, only the soft outer felt is \
         squeezed and the tone is round; played harder, the hammer works into denser felt and \
         the tone brightens. This is how dense the inside is (0: one uniform felt).",
        "Adds Q_core·(u − d)^p_core to the felt force, with Q_core = core_hardness × Stulov's Q0 \
         of the key (whatever the voicing of the top felt). With d = 0 and p_core above the top \
         felt's exponent, the felt is graded: it stiffens continuously (default 2, p_core 7, \
         docs/felt.md). With d > 0 it is a two-layer hammer whose core is reached past d \
         (docs/presence.md).",
    ),
    ("core_p", "How fast the felt stiffens as it is compressed (graded felt), or how abruptly the core resists (two layers).", "Exponent p_core of the inner term Q_core·(u − d)^p_core."),
    (
        "core_depth_bass",
        "Two-layer hammer only: thickness of soft felt over a hard core on the lowest hammer (A0). \
         Thicker: it takes a harder stroke to get the bright bite. 0: a graded felt.",
        "Top-felt depth d (mm) at A0, linear to the middle value at C4. The una corda deepens it.",
    ),
    (
        "core_depth_mid",
        "Two-layer hammer only: thickness of soft felt over the core at middle C.",
        "Depth d (mm) at C4, the joint of the two linear segments (A0–C4, C4–C8).",
    ),
    ("core_depth_treble", "Two-layer hammer only: thickness of soft felt over the core on the highest hammer (C8).", "Depth d (mm) at C8, linear from C4."),
    (
        "p_bass",
        "Makes the bass hammers more nonlinear: rounder when played softly, biting when played \
         hard.",
        "Added to the felt exponent p, growing linearly from 0 at C4 to p_bass at A0.",
    ),
    (
        "p_offset",
        "How fast the felt stiffens as it is squeezed, on every key. Higher values widen the gap \
         between the soft and the loud timbre.",
        "Added to Stulov's exponent p = 3.7 + 0.015·key of the top felt (published values 2–5).",
    ),
    (
        "hysteresis",
        "Felt does not spring back the way it was squeezed: it keeps some of the energy. More \
         hysteresis gives a rounder tone and a softer rebound.",
        "Multiplies Stulov's hysteresis time α in F = Q0·[u^p + α·d(u^p)/dt]. The contact solver \
         uses its linearised form, which only removes energy (passive by construction).",
    ),
    (
        "contact_width_scale",
        "Width of the felt that touches the string. A wide patch cannot excite very short waves \
         on the string: fewer high harmonics, a softer tone.",
        "Scales the contact width (8 mm at A0 to 4 mm at C8). Partial k is struck with \
         sin(kπ·x0)·sinc(kπ·w/2L): a spatial low-pass.",
    ),
    (
        "hammer_mass_scale",
        "Weight of the hammers. Light hammers (fortepianos, small uprights) bounce off the strings \
         sooner: a thinner, quicker attack; heavy ones stay longer on the strings and give a \
         fuller, rounder tone.",
        "Multiplies Stulov's hammer masses (11 g at A0 down to 5.3 g at C8). The contact solver \
         integrates m·x'' = −F, so at a given speed the strike's energy and duration grow with \
         the mass.",
    ),
    (
        "shank_share",
        "The hammer's shank bends. In the treble the short blow meets the hammer head almost \
         alone, which gives the top notes their strength; in the bass the whole hammer follows. \
         Share of the hammer's mass in its shank (0: a rigid hammer).",
        "The head (1 − share of the mass) is linked to the shank's effective mass by a spring and \
         a dashpot; both start at the hammer speed (docs/top_level.md).",
    ),
    ("shank_freq", "How stiff the shank is in the top: the frequency at which the head swings against it, from C6 up.", "Frequency (Hz) of the head-against-shank mode from C6 up, k = m_r·(2πf)² with m_r the reduced mass."),
    (
        "shank_freq_mid",
        "How stiff the shank is up to C5. Too supple, the head swings back against the string as the wave returns from the bridge, and a few keys of the middle buzz at the attack.",
        "Frequency (Hz) of the head-against-shank mode up to C5, geometric to the top's at C6.",
    ),
    ("shank_loss", "Damping of the shank's swing.", "Loss factor of the head-against-shank mode (c = loss·m_r·2πf)."),
    (
        "strike_ratio",
        "Where the hammer hits the string, as a share of its length from the tuning-pin end. Near \
         1/8 it barely excites the 8th partial, which would sound harsh; closer to the end the \
         tone gets brighter and thinner, further in rounder.",
        "Strike point x0 = ratio·L up to C6, moving 0.05 closer to the agraffe by C8. Partial k is \
         struck with sin(kπ·x0/L) (times the felt-width sinc): partials with a node at the strike \
         point are not excited.",
    ),
    // Tuning
    (
        "inharmonicity.a_bass",
        "Steel strings are stiff, so their overtones are a little sharp of exact multiples of \
         the fundamental: this gives the piano its slightly metallic colour. This sets how fast \
         that inharmonicity falls from the lowest key towards the middle.",
        "B(n) = e^(a_bass·n + b_bass) + e^(a_treble·n + b_treble) (Rigaud, David & Daudet 2013), \
         partial k at k·f0·√(1 + B·k²). Fitted on Salamander's Yamaha C5 (rms 0.07 in log B).",
    ),
    ("inharmonicity.b_bass", "Inharmonicity of the bass strings: higher, more 'metallic' and more out-of-tune overtones in the bass.", "Offset of the bass exponential of B(n) (natural log)."),
    (
        "inharmonicity.a_treble",
        "How fast inharmonicity grows towards the top. Short, thick treble strings are very \
         stiff.",
        "Slope of the treble exponential of B(n).",
    ),
    ("inharmonicity.b_treble", "Inharmonicity of the treble strings.", "Offset of the treble exponential of B(n) (natural log)."),
    (
        "stretch.a_t",
        "Tuners stretch the octaves: the treble is tuned sharp and the bass flat of equal \
         temperament, so the octaves sound pure despite the sharp overtones. Amount of stretch \
         in the treble.",
        "Stretch in cents: a_t·e^(k_t·(n − 69)) − a_b·e^(−k_b·(n − 69)) + c. Fitted on Salamander, \
         C8 excluded (rms 2 cents): −16 cents at A0, +25 at C8.",
    ),
    ("stretch.k_t", "How quickly the treble stretch builds up towards the top.", "Rate k_t of the treble exponential (per key)."),
    ("stretch.a_b", "Amount of stretch (flattening) in the bass.", "Amplitude a_b of the bass exponential (cents)."),
    ("stretch.k_b", "How quickly the bass stretch builds up towards the bottom.", "Rate k_b of the bass exponential (per key)."),
    ("stretch.c", "Offset of the whole tuning around A4 (cents).", "Constant term c of the stretch law."),
    (
        "temperament.kind",
        "How the twelve notes of the octave are tuned against each other. Equal temperament (0) makes \
         every key alike; the older ones keep some keys purer and others rougher, as music was \
         played before the 19th century: 1 Werckmeister III, 2 Kirnberger III, 3 Vallotti, \
         4 Young II, 5 quarter-comma meantone, 6 sixth-comma meantone, 7 Pythagorean, 8 custom.",
        "Set by how each fifth of the circle is narrowed from pure (3:2): Werckmeister III, C–G, \
         G–D, D–A and B–F# by ¼ Pythagorean comma; Kirnberger III, C–G … A–E by ¼ syntonic \
         comma and F#–C# by the schisma; Vallotti, F–C … E–B by ⅙ Pythagorean comma; Young II, \
         C–G … B–F# likewise; the meantones, eleven fifths by ¼ or ⅙ syntonic comma and the \
         wolf G#–Eb; Pythagorean, eleven pure fifths and the wolf. A4 keeps its pitch; the \
         stretch adds on top (docs/temperaments.md).",
    ),
    (
        "temperament.key",
        "The note the temperament is built on (0 = C … 11 = B): the keys around it in the circle \
         of fifths sound purest. C for the usual tables.",
        "Shifts the note classes' offsets by this many semitones.",
    ),
    (
        "temperament.cents",
        "A temperament of your own (temperament 8): how far each note, C to B, lies from equal \
         temperament, in cents (a hundredth of a semitone).",
        "Offsets of C, C# … B, laid on `temperament.key`; A is brought back to A4's pitch.",
    ),
    // Strings
    (
        "losses.b1_slope",
        "How much faster notes die away towards the treble, from the strings' own losses (air \
         drag, internal friction). The soundboard adds its own.",
        "Intrinsic decay rate σ = b1_floor·w(n) + e^(b1_slope·n + b1_offset + top(n)) + b2·f² \
         (1/s), the T60 being 6.9/σ. Fitted on the ESPRIT aftersound of Salamander, then lowered \
         on the long sustain (docs/long_sustain.md).",
    ),
    ("losses.b1_offset", "Overall sustain of the notes: lower values ring longer.", "Offset of the log-linear term of σ."),
    (
        "losses.b2",
        "How much faster the high overtones die than the fundamental: higher values make the \
         sound darken sooner as it decays.",
        "Frequency term b2·f² of σ. 6e-8 from eight real grands' sustain (docs/top_treble.md); \
         1e-7, set against Salamander (docs/sustain.md), cut the treble's upper partials short.",
    ),
    (
        "losses.b1_floor",
        "Extra losses of the wound bass strings (copper wire wound around the steel core rubs \
         on itself).",
        "Added to σ below `wound_until`, fading over the octave above. 0.2 /s puts the bass \
         envelopes within 1 dB of Salamander.",
    ),
    ("losses.wound_until", "Highest key with wound strings.", "MIDI note where the b1_floor term starts fading (gone an octave higher)."),
    (
        "losses.top_slope",
        "Extra losses of the top strings, which have no dampers: higher values shorten what \
         rings on after the keys are released, and the notes' own slow tail.",
        "σ's log-linear term gains top_slope·(n − top_from) above top_from. 0 by default: real \
         top strings keep a slow aftersound (docs/top_treble.md); 0.05 (×1.8 at C7) was set \
         against a commercial modelled piano's release ring (docs/release_ring.md).",
    ),
    ("losses.top_from", "Key from which the top strings lose energy faster.", "MIDI note where top_slope starts."),
    (
        "losses.winding",
        "Losses of the lowest strings' copper winding: higher values make the low bass's bright, \
         metallic ring after the attack die away sooner.",
        "Loss factor η: every partial loses ½·η·ω, full at A0 (double-wound strings), fading out \
         linearly to `winding_until`. 0: none (docs/bass_metal.md).",
    ),
    ("losses.winding_until", "Key where the winding losses have faded out.", "MIDI note."),
    (
        "scale.len_c8",
        "Speaking length of the highest string (C8), between its two bearing points.",
        "Lengths grow by `ratio_per_octave` per octave down to `break_note`. This generic 2 m \
         grand scale is a placeholder, not a measured one. With the tension and the pitch it \
         sets each string's mass: μ = T/(2·L·f0)².",
    ),
    ("scale.ratio_per_octave", "How much longer the strings get per octave down (2 would be the ideal, pure length scaling).", "Length ratio per octave from C8 to the break note."),
    (
        "scale.break_note",
        "Where the strings stop lengthening at the ideal rate: below, a grand's case is too short, \
         so the bass strings are shorter and made heavier by winding.",
        "Below this note the length goes linearly to `len_a0`.",
    ),
    ("scale.len_a0", "Speaking length of the lowest string (A0).", "End of the foreshortened bass segment (m)."),
    (
        "scale.tension",
        "Tension of each string (typically 700–900 N). Heavier, tenser strings push harder on \
         the bridge.",
        "The same T on every string. With L it sets μ, the bridge coupling (T/L)·Y, the damper \
         impedance √(T·μ) and the hammer's modal masses.",
    ),
    (
        "unison.bichord_from",
        "Lowest key with two strings (below: one heavy wound string per key).",
        "MIDI note. The strings of a key are coupled through the bridge (Weinreich 1977): double \
         decay and beating.",
    ),
    ("unison.trichord_from", "Lowest key with three strings.", "MIDI note."),
    (
        "unison.width_mid",
        "The strings of a key are never exactly in tune with each other. This slight mistuning \
         gives the piano's sustained, shimmering 'aftersound'; too much and notes sound out of \
         tune or honky-tonk. Spread up to middle C.",
        "Spread between the extreme strings (cents) up to C4, growing quadratically to \
         `width_treble` at C8; each key gets an irregular but fixed pattern. 1 cent puts the \
         aftersound 13–17 dB below the prompt sound (docs/long_sustain.md).",
    ),
    ("unison.width_treble", "Mistuning spread of the strings at the top key (C8).", "Spread (cents) at C8; calibrated on the ESPRIT statistics of Salamander."),
    (
        "unison.width_curve",
        "How the strings' mistuning grows from the middle to the top key: higher keeps the octaves \
         above C4 truer and the top as wide.",
        "Exponent of width_mid + (width_treble − width_mid)·t^curve, t from 0 at C4 to 1 at C8 (2: quadratic).",
    ),
    (
        "unison.hit_spread",
        "The hammer never hits the strings of a key exactly evenly. The uneven part sets them \
         swinging against each other, which feeds the long aftersound.",
        "Relative spread of the force share each string gets (0: equal), a zero-mean pattern \
         rotated per key. Set on the aftersound level of the treble (docs/sustain.md).",
    ),
    (
        "unison.soft_exponent",
        "A soft blow touches the strings of a key less evenly than a hard one, so soft notes \
         keep their upper partials longer. 0: the same unevenness at every speed.",
        "Spread = hit_spread·(soft_speed/v)^exponent, capped at 1 (v: hammer speed). Set on the \
         early decay of the middle's upper partials at pp and ff on eight real grands \
         (docs/release.md).",
    ),
    (
        "unison.height_spread",
        "The strings of a key never lie exactly level: a slow hammer reaches them one after the \
         other.",
        "Height spread (m) of a unison's strings: string j takes no hammer force until \
         h_j·spread/v (h_j in 0…1, irregular per key, v the hammer speed) (docs/release.md).",
    ),
    (
        "unison.load_ramp",
        "Where a key gets a second or a third string, the strings start a little slacker and \
         reach their full tension over this many keys. More: the first double and triple \
         strings of the bass ring longer.",
        "Semitones; the first bichord and trichord carry (n−1)/n of the scale's tension per \
         string, so the key's load on the bridge, Σ √(T·μ), does not jump. 0: full tension \
         everywhere (docs/bass_metal.md).",
    ),
    ("unison.soft_speed", "Hammer speed at which the unevenness is `hit_spread`; slower blows are more uneven.", "Reference speed (m/s) of the soft-blow law."),
    (
        "bridge.g_1k",
        "How easily the strings move the bridge, when no soundboard is modelled. More mobility: \
         louder but shorter notes.",
        "Only used without a soundboard. Bridge admittance G(f) = g_1k·(f/1 kHz)^slope (s/kg), \
         Im Y = b_over_g·G; measured on Salamander from the prompt/aftersound decays.",
    ),
    ("bridge.slope", "How the bridge mobility changes with frequency (no soundboard).", "Exponent of G(f)."),
    ("bridge.b_over_g", "Springiness of the bridge relative to its losses (no soundboard): shifts the strings' pitch slightly and changes their beating.", "Im Y / Re Y."),
    // Polarisation
    (
        "polarization.enabled",
        "Strings vibrate up-and-down and sideways. The sideways motion barely moves the bridge, \
         so it rings much longer: part of the piano's long sustain, even on one-string notes.",
        "Doubles the oscillators of each partial (vertical and horizontal) in the coupled unison \
         system.",
    ),
    (
        "polarization.h_coupling",
        "How much the bridge gives sideways, relative to up-and-down. Low: the sideways motion \
         rings long and quietly.",
        "ε_H/ε_V at C4 (it follows `slope` from there). 0.3 keeps the middle's fundamentals at \
         the real grands' level at 1 s (docs/fundamental_sustain.md).",
    ),
    (
        "polarization.cross",
        "How much energy passes between the two directions of vibration (bridge rocking, oblique \
         string ends): it slowly transfers the loud up-and-down motion into the long sideways one.",
        "ε_X/ε_V. The bridge stays passive only if ε_X² ≤ ε_V·ε_H, so it is clamped at \
         √h_coupling; the default sits at that limit.",
    ),
    ("polarization.detune_cents", "Pitch difference between the sideways and up-and-down vibrations (the string's ends hold it differently in each direction).", "Horizontal minus vertical frequency (cents)."),
    ("polarization.radiation", "How much of the sideways vibration is heard.", "Output weight of the horizontal bridge force (vertical = 1)."),
    ("polarization.hammer_tilt", "How much the hammer also pushes the strings sideways.", "Share of each string's hammer force on its horizontal oscillator."),
    (
        "polarization.smooth",
        "Whether the sideways vibration feels the soundboard's resonances. Higher: a note sitting on \
         a board resonance keeps its long, quiet sideways ring instead of losing it with the loud \
         start.",
        "ε_H = h·((1 − s)·ε_V + s·ε_ref), ε_ref the board's smooth point mobility (infinite plate) \
         × T/L. ε_X = (cross/√h)·√(ε_V·ε_H), clamped passive. 0: ε_H follows the key's vertical \
         admittance.",
    ),
    (
        "polarization.slope",
        "How the sideways give of the bridge changes with pitch. Positive: the bridge rocks more \
         easily for high notes.",
        "ε_H grows as (f / 261.63 Hz)^slope; 1 is a bridge rocking on an infinite plate (moment \
         mobility proportional to f).",
    ),
    (
        "polarization.slope_from",
        "Lowest pitch the sideways slope follows; below it the bridge's sideways give stays as there.",
        "Hz; ε_H uses max(f, slope_from) in its slope. 0: none.",
    ),
    // Soundboard
    (
        "soundboard.a",
        "Length of the soundboard, along the strings. The soundboard is the large thin spruce \
         panel under the strings: it turns their vibration into sound.",
        "The board is an orthotropic plate a × b clamped in the rim (Warburton's approximation, \
         index m to m + ½), modal: each mode is a damped oscillator driven by the bridge forces \
         and radiating to the mics by the Rayleigh integral.",
    ),
    ("soundboard.b", "Width of the soundboard, across the keyboard.", "Plate size b (m). The modes' frequencies scale as 1/a² and 1/b²."),
    (
        "soundboard.rho_h",
        "Mass of the soundboard per square metre, ribs and bridges included. Heavier: harder for \
         the strings to move, longer but quieter notes.",
        "Surface mass ρh. ρh/D sets the modal density (≈ 1 mode per 20 Hz, Ege 2013), ρh·D the \
         mean mobility 1/(8·√(D·ρh)), calibrated on the decay statistics of Salamander \
         (docs/phase4.md).",
    ),
    ("soundboard.dx", "Stiffness of the board along the strings (the spruce grain runs across the ribs).", "Bending stiffness Dx (N·m) along a."),
    ("soundboard.dy", "Stiffness of the board across the strings.", "Bending stiffness Dy (N·m) along b; H = √(Dx·Dy)."),
    (
        "soundboard.loss_factor",
        "Internal damping of the soundboard. More damping: smoother, less resonant board, \
         shorter notes.",
        "Loss factor η of the modes (Ege et al. 2013: ≈ 2 %), growing with frequency as \
         η·(1 + f/loss_corner).",
    ),
    (
        "soundboard.loss_corner",
        "Above this frequency the board damps more and more. Lower values dull the high \
         frequencies.",
        "Corner of η(f) = η·(1 + f/loss_corner). 700 Hz muffled the 2–8 kHz bands \
         (docs/brightness.md).",
    ),
    (
        "soundboard.loss_low_corner",
        "Below this frequency the board's low modes lose more energy, through its edge into the \
         rim and the case. Higher values: bass fundamentals that fade faster.",
        "η(f) is multiplied by 1 + (loss_low_corner/f)²; 0: no extra loss.",
    ),
    (
        "soundboard.edge_loss",
        "Energy the strings lose into the rim and the case where their bridge runs near the \
         board's edge (the bass). Higher: bass notes that fade faster, the middle untouched.",
        "Conductance (s/kg) added to the bridge mobility, × e^(−d/edge_depth) with d the bridge \
         point's distance to the nearest edge, × 1/(1 + (f/edge_corner)²); it radiates nothing. \
         0: none.",
    ),
    ("soundboard.edge_depth", "How far from the board's edge the rim loss still reaches.", "Decay length (m) of the edge loss's weight e^(−d/edge_depth)."),
    ("soundboard.edge_corner", "Above this frequency the rim loss fades.", "Corner (Hz) of 1/(1 + (f/edge_corner)²)."),
    (
        "soundboard.string_load",
        "The strings at rest under their dampers soak up some of the board's vibration. They \
         smooth its resonances below ≈ 1 kHz, so that neighbouring keys sound more alike. 0: \
         none, 1: the physical estimate.",
        "Each damped string loads the bridge with its characteristic impedance √(T·μ) (a lossy \
         string's input resistance averages to it): mode n gains a damping rate \
         ½·Σ√(T·μ)·Φ_n² over the damped keys, × this scale. At 1: η ≈ 0.07 at 100–200 Hz, \
         0.04 at 200–400 Hz, 0.034 at 400–800 Hz (docs/evenness.md).",
    ),
    (
        "soundboard.string_load_free",
        "How much the strings still calm the soundboard when the sustain pedal lifts the dampers. \
         Free strings take the board's vibration too, and sing it back as resonance. 0: with the \
         pedal down the board rings on its own, a drone on its lowest resonance.",
        "Share of the damped strings' load kept while the pedal lifts the dampers (1: a free \
         string's input resistance averaged over frequency, √(T·μ) as for a damped one). The \
         sympathetic coupling is one-way: this load is the energy the listening strings take from \
         the board (docs/pedal_drone.md).",
    ),
    (
        "soundboard.exact_below",
        "Below this frequency every key drives the soundboard at its own point on the bridge, \
         not at the point standing for its group of keys: neighbouring keys sound more alike. \
         About 4 % more CPU at 1000 Hz. 0: off.",
        "The board modes below it run in a bank of their own, driven through the 16 principal \
         profiles of the keys' drive shapes (99.98 % of them below 1 kHz); each key feeds them \
         with its own weights. The others keep the 16 bridge regions; the sympathetic forces \
         keep them all (docs/evenness.md).",
    ),
    ("soundboard.bridge_mass", "Mass of the bridge where a string crosses it: heavier filters the highs reaching the board.", "Local mass m: the strings see Y/(1 + iω·m·Y), the board receives F/(1 + iω·m·Y). 0: none."),
    (
        "soundboard.series_stiffness",
        "Springiness between the string and the bridge (the bridge pins and cap).",
        "Stiffness k in series: adds iω/k to the admittance the strings see.",
    ),
    (
        "soundboard.bridge_stiffness",
        "Stiffness of the bridge, the hardwood beam glued on the board where the strings rest. \
         A stiff bridge spreads each string's push along its length, which smooths the high \
         frequencies.",
        "EI (N·m²): a string's force reaches the board spread (Hann window) over `bridge_spread` \
         bending wavelengths 2π·(EI/(μ·ω²))^¼, capped at 0.5 m. EI = 0 drives the board at a \
         point.",
    ),
    ("soundboard.bridge_line_mass", "Mass of the bridge per metre.", "Line mass μ (kg/m) in the bridge's bending wavelength."),
    ("soundboard.bridge_spread", "Over how much of the bridge a string's push is spread.", "Spread length in bridge bending wavelengths; set with the decays of the upper partials (docs/sustain.md)."),
    (
        "soundboard.impedance_slope",
        "The board is thicker and more ribbed under the treble, so it moves less there. Higher \
         values: a quieter treble that sustains longer.",
        "The impedance under the bridge grows as e^(slope·(key − 60)) above C4, applied to the \
         mode shapes the strings drive. 0.02 (×2.4 at C8) from eight real grands' top-treble \
         decays (docs/top_treble.md); 0.035 was set on Salamander's alone.",
    ),
    (
        "soundboard.impedance_slope_bass",
        "The same towards the bass. A bass side that moves too freely booms like a barrel.",
        "Slope below C4; −0.03 makes the bass side stiffer (docs/bass_radiation.md).",
    ),
    ("soundboard.f_max", "Highest soundboard resonance modelled. Lower values save CPU but lose brilliance.", "Board modes above it are dropped (Hz)."),
    (
        "soundboard.piston_freq",
        "The whole case (board, rim, body) also moves as a block on its legs, at a low \
         frequency: it absorbs the deep bass.",
        "A uniform, heavily damped 'piston' mode below the first clamped one (Hz, 0: none) \
         (docs/bass.md).",
    ),
    ("soundboard.piston_loss", "Damping of that whole-case motion.", "Loss factor of the piston mode."),
    ("soundboard.piston_mass", "Mass moving in that whole-case motion. Lighter: it takes more energy from the bass notes.", "Effective mass (kg); set on the late sustain of the bass (docs/long_sustain.md)."),
    (
        "soundboard.piston_radiation",
        "How much of the whole-case motion is heard (little: air pushed above is returned below).",
        "Radiation relative to a bare board moving alone.",
    ),
    (
        "soundboard.crown_freq",
        "The soundboard is slightly arched (crowned) and pressed down by the strings, which \
         stiffens it like a shell. More crown: its lowest resonances move up.",
        "Elastic foundation: ω² becomes ω² + (2π·crown_freq)² (Hz, 0: flat). Set on the third-octave \
         balance of the bass keys (docs/bass_radiation.md).",
    ),
    (
        "soundboard.treble_bridge",
        "Where the main bridge lies on the soundboard. Where a string drives the board decides \
         which resonances it excites.",
        "Smooth curve (Catmull-Rom) through 5 points [x, y], fractions of the board size, from C8 \
         down to the first note of the main bridge; the points fall at equal steps of keys. The treble end sits in the clamped \
         corner, as on real grands. Board coordinates: x along the strings, from the keyboard \
         end of the board towards the tail; y across, from the treble side (0) to the bass \
         side.",
    ),
    ("soundboard.bass_bridge", "Where the separate bass bridge lies, nearer the middle of the board.", "Its 2 ends [x, y] (fractions of the board size), from the note below `main_bridge_from` down to A0."),
    (
        "soundboard.bass_bridge_bow",
        "How much the bass bridge curves. Real bass bridges are often curved.",
        "Arc of a circle through the bass bridge's two ends, this far (m) from their chord at its \
         middle; positive towards the tail. The keys stay at equal steps along it. 0: straight; \
         a preset that does not set it gets a straight bridge.",
    ),
    (
        "soundboard.main_string_slant",
        "How much the strings of the main bridge lean towards the bass side as they run back from \
         the keys, as on real grands: the tenor's most, the treble's hardly. The bass strings \
         cross over them leaning the other way.",
        "Angle (degrees) from the keyboard's normal at the main bridge's lowest key, falling in \
         equal steps of keys to 0 at C8 (the strings fan). Each string turns about its key's place \
         on the keys' line, 3 cm in front of the board, keeping its length: the main bridge's \
         points move towards the bass side by the string's length times its angle's sine. The \
         frame's agraffes stay at the keys. A string leans no further than keeps its bridge point \
         5 cm inside the board's bass side and front edge; negative angles read as 0. 0: \
         upright strings; a preset that does not set it gets upright strings.",
    ),
    ("soundboard.main_bridge_from", "Lowest key on the main bridge; the keys below are on the bass bridge.", "MIDI note."),
    (
        "soundboard.outline",
        "The soundboard's shape. A grand's board follows the curved side of its case and narrows \
         towards the tail; all points at zero: a plain rectangle.",
        "Polygon of 10 points [x, y], fractions of the board size (as the bridges), inside the \
         a × b rectangle; unused points repeat the last one. The rim holds the board along it: \
         its resonances below `outline_below` are the outlined plate's own (docs/outline.md).",
    ),
    (
        "soundboard.rib_spacing",
        "Distance between the ribs glued under the soundboard, across its length. 0: no ribs, a \
         plain plate. With ribs, the high resonances stay between two ribs instead of spreading \
         over the whole board, and reach the player's ears from every angle.",
        "Ribs parallel to the short side, equally spaced along the length (m). Their stiffness \
         is added where they lie and taken away between them (the smeared share is already in \
         `dy`): nothing changes for the long waves; the modes above the frequency where half a \
         bending wave fits between two ribs are confined (docs/board_radiation.md). Acts on the \
         outlined board's own modes, below `outline_below`.",
    ),
    (
        "soundboard.rib_share",
        "How much of the soundboard's stiffness across its length sits in the ribs rather than in \
         the wood between them. 0: no ribs.",
        "Share of `dy` carried by the ribs: each rib's bending stiffness is `rib_share` × `dy` × \
         `rib_spacing` (N·m²).",
    ),
    (
        "soundboard.bridge_beam",
        "The bridges as beams glued on the soundboard: their weight and stiffness along their \
         curve make neighbouring points of a bridge move together. 0: off.",
        "Factor on `bridge_stiffness` (EI, N·m²) and `bridge_line_mass` (kg/m) added along the main \
         and the bass bridge in the outlined board's own modes, below `outline_below`.",
    ),
    (
        "soundboard.rib_jitter",
        "How irregular the ribs are: a real board's ribs are not equally spaced nor alike, which \
         confines its high vibrations between them. 0: equally spaced and alike.",
        "Each rib moves by up to ± half this share of `rib_spacing` and its stiffness varies by up \
         to ± this share (a fixed pseudo-random sequence, mean kept).",
    ),
    (
        "soundboard.scatter",
        "How much of the soundboard's sound is scattered in every direction by what lies over it: \
         the strings, the iron frame, the dampers and the bridges. The board alone sends its highs \
         straight up; a listener at the keyboard hears them scattered. 0: none.",
        "Gain of a layer of secondary sources over the board, each re-radiating the board's \
         velocity under it as a small obstacle does, (f/scatter_corner)² / (1 + (f/scatter_corner)²), \
         at heights spread about `scatter_height` so that they add in power above ≈ 2 kHz \
         (docs/board_radiation.md).",
    ),
    ("soundboard.scatter_corner", "Frequency above which the scattering is full.", "Hz: the obstacles' size against the wavelength, (f/fc)²/(1 + (f/fc)²)."),
    ("soundboard.scatter_step", "Spacing of the scattering sources over the board.", "m; 0.1 gives about 180 sources on a 2 m grand."),
    ("soundboard.scatter_height", "Mean height of the scattering layer over the board.", "m; the heights are spread from 0.4 to 1.6 times it."),
    ("soundboard.scatter_shadow", "How much of the board's direct sound the scattering layer takes away above it, to send it elsewhere.", "Share of the direct field's power intercepted at full scattering (same frequency law), 0–1; the front face and its lid image of a point above the board are scaled by √(1 − share)."),
    (
        "soundboard.outline_below",
        "Up to this frequency the soundboard resonates with its own shape (see Outline); above, \
         as the rectangle around it.",
        "Rayleigh–Ritz on the rectangle's modes below 4× this frequency, a stiff ground holding \
         the board outside the outline. Above it the rectangle's modes stay, radiating from \
         inside the outline only (Hz).",
    ),
    (
        "soundboard.mics",
        "Where the two microphones stand: the first gives the left channel, the second the \
         right. Close to the board the sound is detailed and dry; further away the board blends \
         and the room takes over.",
        "[mic][x, y, z] in metres. Board coordinates: x along the strings, from the keyboard end \
         of the board towards the tail; y across, from the treble side (0) to the bass side; z \
         above the board. Each mic hears every board mode by the Rayleigh integral (plus lid \
         image, frame and rim), then the room's reflections. Changing them rebuilds the \
         radiation weights.",
    ),
    (
        "soundboard.mics_upright",
        "Where the two microphones stand when the piano is an upright: in front of its \
         soundboard. Each listening set-up places them for both kinds of piano.",
        "Used instead of soundboard.mics on a vertical board: x down from the board's top, y \
         across (bass at y = b, the listener's left), z in front of the board; fractions of a \
         and b with mics_relative.",
    ),
    (
        "soundboard.mics_relative",
        "Microphone positions given as shares of the soundboard's length and width instead of \
         metres, so that a microphone set-up follows the board when the instrument changes size.",
        "x and y are multiplied by the board's a and b when the instrument is built; the height \
         stays in metres. The listening presets use it.",
    ),
    (
        "soundboard.mic_gain_db",
        "Level of the microphone set-up, on top of the instrument's own output gain: close \
         microphones hear the piano louder, so a close set-up turns itself down.",
        "Gain (dB) applied to the radiated sound when the instrument is built, kept apart from \
         output_gain_db so that a listening preset and an instrument preset combine.",
    ),
    (
        "soundboard.mic_pattern",
        "Pick-up pattern of the microphones, aimed at the piano: 1 hears all around (omni), 0.5 is a \
         cardioid, which hears the room about 5 dB less than the piano in front of it.",
        "Pattern a + (1 − a)·cos θ. The direct sound arrives on axis; the room's reflections and tail \
         come from every direction and are heard with the random-incidence efficiency \
         a² + (1 − a)²/3 (1/3 for a cardioid).",
    ),
    (
        "soundboard.radiate",
        "Off: you hear the force on the bridge instead of the sound in the air (for analysis).",
        "False skips the Rayleigh radiation; the board still sets the string coupling.",
    ),
    ("soundboard.grid_step", "Accuracy of the radiation computation (smaller: more accurate, slower rebuild).", "Grid step (m) of the Rayleigh integral over the board, per mode and mic."),
    (
        "soundboard.back_depth",
        "The board radiates from both faces; the underside, in opposite phase, partly cancels \
         the front at low frequencies. This is how far below the sound of the underside comes \
         from.",
        "Depth (m) of the back face's image: p = iωρ0/(2π)·∫ v·(e^(−ikR)/R − β·e^(−ikR')/R') dS.",
    ),
    ("soundboard.back_gain", "How much of the underside's sound reaches the mics (rim and keybed shield it partly). 0: an infinite baffle.", "β in the Rayleigh integral; calibrated on the spectral tilt of Salamander."),
    (
        "soundboard.wall_distance",
        "An upright usually stands against a wall: the sound of the back of its soundboard \
         bounces off it and comes back into the room. Distance from the board to the wall; 0: no \
         wall.",
        "Adds the back face's image in the wall, path R'' with the back-face depth plus \
         2·distance, gain −β·r.",
    ),
    ("soundboard.wall_reflection", "How much sound the wall behind the piano reflects (1: hard wall).", "Pressure reflection coefficient r of the wall image."),
    (
        "soundboard.vertical",
        "On for an upright: the soundboard stands vertically, its top at the board's length above \
         the floor, its string side towards the room (and against the room's wall when there is \
         a wall behind it). It places the piano and the microphones in the room.",
        "Board (x, y, z) to room (piano.x + y, y_wall + z, 0.25 m + a − x), y_wall = wall distance \
         (then the room's first reflection off y = 0 is left to the board's own wall image) or \
         piano.y. The board's radiation is unchanged.",
    ),
    (
        "soundboard.case_open",
        "An upright's case closes the front of its soundboard: action, keys and panels. Share of \
         that side's sound that gets out through the openings; 1: no case, or panels removed. \
         Lower: a darker, more boxed-in tone.",
        "Gain of the front face's paths: open + (1 − open)·τ(f), τ = 1/√(1 + (ω·m/2ρc)²) the \
         panels' mass-law transmission. The back face and the wall are not affected.",
    ),
    ("soundboard.case_panel_mass", "Weight of the case panels per square metre: heavier panels let less sound through.", "Surface mass m (kg/m²) in the mass law; 6 ≈ 1 cm of wood."),
    (
        "soundboard.lid_angle",
        "How far the lid is open (0: removed). An open lid reflects the sound towards the audience \
         side and shadows the mics behind it.",
        "The lid is a reflecting rectangle hinged on the bass side (y = b), opening towards y < 0. \
         Each board cell gets an image path through it when the reflection hits the lid; mics \
         behind it get the transmitted share. No diffraction.",
    ),
    ("soundboard.lid_hinge_height", "Height of the lid's hinge above the soundboard.", "z of the hinge line (m)."),
    ("soundboard.lid_reflection", "How much sound the lid reflects.", "Pressure reflection coefficient of the lid image."),
    ("soundboard.lid_transmission", "How much sound passes through the lid to a microphone behind it.", "Gain of the direct path when the lid shadows it."),
    (
        "soundboard.lid_shut",
        "Lid shut on the case (1) rather than open at its angle or taken away (0). The shut lid boxes \
         the board's top face in: the tone loses its highs, the bass hardly changes.",
        "Gain on the front face's paths: 1 − s + s·(leak + (1 − leak)/√(1 + (f/fc)²)), fc = c/2h with \
         h the hinge height (the box's height resonance, ≈ 690 Hz). At 1 no open lid, whatever the \
         angle. Grands only.",
    ),
    (
        "soundboard.lid_leak",
        "How much of the board's top sound still gets out of the shut lid in the highs, by the gaps \
         and the opening behind the music desk.",
        "Floor of the shut lid's gain above fc (pressure ratio).",
    ),
    // Musician's controls
    ("macros.tone", "Soft to bright: how hard the hammers' felt is.", "Top felt and inner felt hardness × 4^v and × 12^v (v from −1 to +1, 0: the preset). C3's attack centroid moves ±18 %, C5's ±7 % (docs/macros.md)."),
    ("macros.touch", "Even to contrasted: how far soft and loud playing lie apart, mezzo forte unchanged.", "The hammer speeds' log-distance from velocity 60's × 1.5^v: C4 velocity 100 − 30 changes by −6 to +9 dB."),
    ("macros.sustain", "Short to long notes.", "String losses (b1, its floor, b2) and the board's edge loss × 3^−v: C5 at 2 s −14 to +5 dB (beyond, the board sets the sustain)."),
    ("macros.resonance", "Dry to rich: how much the strings ring along with what is played, pedal down, and how many top strings have no damper.", "Sympathetic gain × 2^v, highest damper 4·v keys lower: the halo of a pedalled C4 ±5.5 dB."),
    ("macros.release", "Clean to soft note endings.", "Damper strength × 3^−v, fall time × 3^(v/2): C4's drop 0.25 s after the key-up +4 to −7 dB."),
    ("macros.unisons", "Pure to lively: how far apart the strings of each note are tuned, and how unevenly the hammer meets them.", "Unison spread × 3^v (× 1.6^v towards pure: tighter unisons lose the slow modes that carry the sustain), hit spread × 1.5^v. C5 over 1–2 s −3.7 to +5.7 dB."),
    ("macros.mechanism", "Discreet to present action noises: the keys' knock, the dampers, the pedal. Four tenths up, the level of real grands' recordings.", "noise.action, keybed, damper_mass and pedal × 8^v (±18 dB); the defaults are 7.5 dB below the level measured on eight real grands, which +0.42 reaches."),
    ("macros.size", "Baby grand to concert grand.", "Bass string length × 1.25^v (bass inharmonicity as 1/L²), board and frame × 1.12^v, hammer mass × 1.1^v."),
    ("macros.lid", "Closed to open lid.", "Lid angle from the preset's towards 0 (v = −1) or 45° (v = +1), and its closure towards shut (v = −1) or none (v = +1): at −1 the lid is shut."),
    ("macros.space", "Close to roomy: how much of the room's reverberation is heard.", "Room tail × 3^v (about ±8 dB)."),
    ("macros.stretch", "Strict to stretched tuning: how wide the octaves are tuned.", "The stretch law × (1 + v): equal temperament at −1, twice the preset's stretch at +1."),
    // Dampers
    (
        "damper.position",
        "Where the felt dampers rest on the strings, near the keyboard end. They silence a note \
         when its key is released.",
        "Fraction of the string length from the agraffe. A damper is a dashpot adding to partial \
         k a decay rate proportional to sin²(kπx/L) averaged over the pad (S_k).",
    ),
    (
        "damper.behind_strike",
        "How far behind the hammer the dampers rest. On a grand they stand in a straight line \
         behind the hammers, so they sit further along the short treble strings.",
        "Distance (m) along the strings from the strike point to the pad's middle; 0 keeps \
         `position` on every string (presets saved before it).",
    ),
    ("damper.width", "Length of string the felt pad covers.", "Pad width (m) in the average of sin² (partials with a node under the pad are still damped by the rest of it)."),
    (
        "damper.node_floor",
        "Some overtones have a still point right under the damper and would escape it; the \
         crushed felt still catches them. Higher: cleaner note endings.",
        "Share of the full rate every partial gets at least. Set on the release of the bass \
         (docs/release_ring.md).",
    ),
    (
        "damper.strength",
        "How firmly the dampers stop the strings. Too soft and notes linger after release; too \
         hard and the felt bounces the vibration back instead of absorbing it.",
        "c/(2·Z0), Z0 = √(T·μ): σ_k = c/(μL)·S_k·(2Z0/(c + 2Z0))². 1 matches the string's \
         impedance (most absorbing). S_k: mean of sin²(kπx/L) over the pad. 0.1 set on the \
         release of a Disklavier grand's notes (docs/release.md).",
    ),
    (
        "damper.horizontal",
        "The felt rests on top of the strings: it stops their up-and-down motion but only \
         rubs against the sideways one, which leaves a faint tail after the release.",
        "Share of the damper rate the horizontal polarisation gets (first order in each coupled \
         mode: Σ_j w_j·V_jm²).",
    ),
    (
        "damper.contact_exponent",
        "How the damping grows as the felt settles on the strings. Higher: lightly touching \
         dampers barely stop the sound, which gives a usable half pedal and softer note endings.",
        "The dashpot grows as contact^exponent (felt compression). With 1, half contact already \
         gave 89 % of the full rate.",
    ),
    ("damper.highest_note", "Highest key with a damper. The short top strings have none and ring until they die.", "MIDI note: D6 by default, from the top notes' sustain on eight real grands (docs/top_treble.md); F#6 was taken from a commercial modelled piano."),
    (
        "damper.pedal_free",
        "Travel of the sustain pedal before it starts lifting the dampers. Pianists rest their \
         foot on the pedal, so a little travel does nothing.",
        "Pedal position (CC 64, 0–1) where the contact starts dropping from 1.",
    ),
    ("damper.pedal_contact", "Pedal travel at which the dampers have left the strings.", "Contact = ((contact − travel)/(contact − free))^curve, clamped to 0–1."),
    ("damper.pedal_curve", "Shape of the half pedal between those two points: higher values make the dampers leave the strings sooner.", "Exponent of the contact curve."),
    ("damper.fall_time", "How quickly a damper falls back on the strings when a key is released.", "Time constant (s) of the contact moving towards 1."),
    ("damper.lift_time", "How quickly a damper lifts off when a key is pressed or the pedal goes down.", "Time constant (s) of the contact moving towards 0."),
    // Sympathy
    (
        "sympathetic.enabled",
        "With the dampers lifted, the other strings vibrate in sympathy with the notes played: \
         the halo of a pedalled piano.",
        "One-way (Bank, Zambon & Fontana 2010): each undamped key gets listening strings driven \
         by the bridge motion that the other keys cause; they do not move the bridge back, so it \
         is stable for any gain.",
    ),
    ("sympathetic.max_freq", "Highest overtone of the sympathetic strings (lower: less CPU).", "Listening strings run their partials below this frequency (Hz)."),
    (
        "sympathetic.gain",
        "Strength of the sympathetic resonance.",
        "Scale on the bridge-to-string gain. 1 is the one-way physical value, which misses the \
         braking of the bridge by the strings ringing at its frequency: 0.6 matches a Disklavier \
         grand's pedal halo in the middle (tools/pedal_bench.py, docs/pedal_drone.md).",
    ),
    ("sympathetic.self_cancel", "Keeps a key from resonating with its own vibration through the bridge.", "Cancels each key's own contribution to its bridge input (already in its coupled modes)."),
    (
        "sympathetic.brake",
        "Strings ringing at the same pitch hold the bridge back, as on a real piano.",
        "Divides each listening mode's drive by 1 + Σ Y·Z over the other free strings near its frequency (the two-way coupling to first order); undamped keys alone with the pedal up, every key with it down.",
    ),
    // Nonlinearity
    (
        "nonlinear.enabled",
        "Struck hard, a string stretches as it swings, which adds 'phantom' overtones: the growl \
         and brightness of a loud bass.",
        "Geometric nonlinearity (Bank & Sujbert 2005): the transverse partials drive the \
         longitudinal modes with their sum and difference frequencies; the longitudinal force on \
         the bridge reaches the board. Grows as the amplitude squared; one-way.",
    ),
    ("nonlinear.young", "Stiffness of the steel of the strings.", "Young's modulus E (Pa); the core diameter follows from B: B = π³·E·d⁴/(64·T·L²), EA = E·π·d²/4."),
    ("nonlinear.max_note", "Highest key computed: above, the phantom overtones are too high to matter.", "MIDI note (E4: its first longitudinal mode is above 4 kHz)."),
    ("nonlinear.max_partial_hz", "Highest overtone taking part (CPU).", "Transverse partials kept in the products (Hz)."),
    ("nonlinear.max_long_hz", "Highest stretching vibration kept (CPU).", "Longitudinal modes kept (Hz)."),
    ("nonlinear.long_loss", "Damping of the stretching vibrations.", "Loss factor of the longitudinal modes."),
    ("nonlinear.coupling", "Loudness of the phantom overtones.", "Share of the longitudinal force at the bridge driving the board; ≈ −28 dB below the partials in the bass at ff (tools/phantom_levels.py)."),
    (
        "nonlinear.coupling_corner",
        "Slow changes of string tension barely move the board; this filters them out.",
        "First-order high-pass (Hz) on the longitudinal force: it bends the board through its \
         moment about the bridge foot, which grows with the board's wavenumber.",
    ),
    // Noises
    (
        "noise.keybed",
        "The thump of the key hitting the keybed, heard through the case. Grows with how hard the \
         key is struck.",
        "Peak force (N per m/s of hammer speed) of a half-sine knock into the board at the key's \
         bridge region, when the hammer strikes.",
    ),
    (
        "noise.damper_mass",
        "The soft thud of a damper landing on a ringing string. Heavier: a more audible thud.",
        "Effective felt mass (kg): a momentum kick mass·speed into the string modes at the damper \
         point, spread over `damper_landing`.",
    ),
    ("noise.damper_speed", "How fast the dampers fall on the strings (scaled by the release velocity of the key).", "Fall speed (m/s) at release velocity 64; the pedal's dampers fall slower."),
    ("noise.damper_landing", "How soft the damper landing is: longer is a duller thud.", "Duration (s) of the half-sine impulse; its spectrum falls above 1/duration."),
    ("noise.pedal", "The knock of the sustain pedal mechanism, heard through the case.", "Peak force (N) into the board for a full-speed pedal move, split between two regions near the middle of the keyboard."),
    ("noise.knock_time", "Duration of the keybed and pedal knocks: longer is duller.", "Length (s) of the half-sine knock."),
    (
        "noise.action",
        "The clack of the action itself, heard directly: the key hitting the keybed, the hammer \
         shank and the keyframe. It gives the attack its percussive edge, most audible in the treble.",
        "Peak pressure (Pa) at the microphones per m/s of hammer speed, from a bank of 24 damped \
         resonances (80 Hz–2.6 kHz) shared by all keys, each key with its own amplitudes. 0.05 \
         matches the knock below the top keys' fundamentals on eight real grands; the default \
         is 7.5 dB below it (docs/action_noise.md).",
    ),
    ("noise.action_time", "How long the action's clack rings.", "Decay time T60 (s) of the resonances at 500 Hz, shorter above as (500/f)^0.3."),
    ("noise.action_delay", "How long after the hammer meets the string the action's clack comes.", "Delay (s) of the knock after the strike: the key bottoms just after the hammer reaches the string."),
    // Room
    ("room.enabled", "The hall around the piano: wall reflections and reverberation.", "Image sources for the early reflections, a 16-line feedback delay network for the tail."),
    (
        "room.size",
        "Length, width and height of the room. Larger rooms: later reflections, longer reverb.",
        "Shoebox [x, y, z] (m). Sabine T60 = 0.161·V/(S·α), air absorption added in the high band.",
    ),
    (
        "room.piano",
        "Where the piano stands in the room: near a wall, its reflections come early and strong.",
        "Position [x, y, z] (m) of the soundboard's corner (0, 0) in the room; z is the board's \
         height. The reflections treat the piano as a point source at the board's centre.",
    ),
    ("room.absorption_low", "How much the walls absorb the low frequencies (≈ 125 Hz). Lower: longer, boomier reverb.", "Mean absorption coefficient α at low frequency."),
    ("room.absorption_high", "How much the walls absorb the high frequencies (≈ 4 kHz). Higher: a darker reverb.", "Mean absorption coefficient α at high frequency."),
    ("room.order", "How many wall bounces are computed as distinct echoes before the reverb takes over.", "Image-source order; each order gets its own wall low-pass."),
    (
        "room.directivity",
        "How much the piano beams its sound towards the microphones rather than all around: the \
         more it does, the less it fills the room. 0 (default) sets it from the piano: a grand \
         with its lid open beams the most, an upright standing free the least.",
        "Directivity factor Q: the room is driven by an omnidirectional source 1/Q as powerful as \
         the level at the mics suggests. 0: 3 for a grand with its lid open, 2 with the lid \
         closed or removed, 2 for an upright against a wall, 1.5 standing free.",
    ),
    ("room.tail_gain", "Level of the reverb tail (1: physical).", "Scales the FDN output, set by default so the reverberant energy is (r/r_c)² times the direct, r_c = 0.057·√(V/T60)."),
    // Frame
    (
        "frame.enabled",
        "The cast-iron frame (plate) that holds the strings' tension, nearly 20 tonnes. It rings a little \
         too, adding metallic colour.",
        "Isotropic clamped plate (same modal machinery as the board), driven at the agraffes by \
         the strings' force and radiating from its own height; one-way.",
    ),
    (
        "frame.coupling",
        "Whether the strings feel the iron frame at their far end. A partial that falls on one of the \
         frame's resonances gives it its energy and rings shorter. 0: the frame only listens.",
        "Factor on the frame's admittance at each key's agraffe, added to the bridge's in the strings' \
         coupled modes (1 = physical).",
    ),
    ("frame.a", "Length of the iron frame.", "Plate size (m); it starts at `offset_x` in board coordinates, over the pin block."),
    ("frame.b", "Width of the iron frame.", "Plate size (m)."),
    ("frame.offset_x", "Where the frame starts relative to the soundboard (negative: in front of it, over the tuning pins).", "x of the frame's corner in board coordinates (m)."),
    ("frame.agraffe_x", "Where the strings' far end (the agraffes, near the tuning pins) sits on the frame.", "x of the agraffe line in frame coordinates (m)."),
    ("frame.height", "Height of the frame above the soundboard.", "z offset (m) of its radiation."),
    ("frame.rho_h", "Mass of the frame per square metre (about 150 kg in all).", "Surface mass (kg/m², holes included)."),
    ("frame.stiffness", "Stiffness of the frame: higher raises its resonances.", "Bending stiffness (N·m); the default gives a first mode near 150 Hz."),
    ("frame.loss_factor", "Damping of the frame: iron rings long, but bolted and loaded it is damped more.", "Loss factor (1 %, an estimate)."),
    ("frame.back_depth", "Distance from which the frame's underside radiates.", "Back-face image depth (m), as for the board."),
    ("frame.back_gain", "How much of the frame's underside reaches the mics. The open-work frame lets the two sides cancel.", "Back-face gain."),
    ("frame.solidity", "Share of the frame's outline that is metal (the rest is openings).", "Only that share radiates."),
    ("frame.f_max", "Highest frame resonance modelled (CPU).", "Frame modes kept (Hz)."),
    ("frame.gain", "Loudness of the frame's sound (1: physical).", "Scales its radiated pressure."),
    // Rim
    (
        "rim.enabled",
        "The rim, the thick curved wooden wall around the soundboard. The board's edges shake \
         it and it radiates too, mostly low and mid frequencies.",
        "A ring of the board's perimeter in out-of-plane bending coupled to torsion (Love), \
         driven by the board's edge forces (Kirchhoff shear and corner forces); its four faces \
         radiate. One-way.",
    ),
    ("rim.height", "Height of the rim wall.", "Wall height (m)."),
    ("rim.thickness", "Thickness of the rim wall.", "Wall thickness (m)."),
    ("rim.top", "How far the rim rises above the soundboard.", "Top of the wall above the board plane (m)."),
    ("rim.young", "Stiffness of the rim's laminated maple, along the wall.", "Young's modulus (Pa)."),
    ("rim.shear", "Resistance of the wall to twisting.", "Torsional shear modulus (Pa)."),
    ("rim.density", "Density of the rim's wood.", "kg/m³."),
    ("rim.loss_factor", "Damping of the rim.", "Loss factor of its modes."),
    ("rim.k_max", "Number of rim vibration patterns around the case (CPU).", "Highest circumferential order."),
    ("rim.f_max", "Highest rim resonance modelled (CPU).", "Rim modes kept (Hz)."),
    ("rim.couple_f_max", "Highest soundboard resonance allowed to shake the rim.", "Board modes above it are not projected on the rim (Hz)."),
    ("rim.inner_gain", "How much the inner face of the rim, half hidden under the board, is heard.", "Radiation gain of the inner face."),
    ("rim.gain", "Loudness of the rim's sound (1: physical).", "Scales its radiated pressure."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parameter_has_help() {
        let reg = crate::Registry::new();
        let missing: Vec<&str> = reg.params().iter().map(|p| p.path.as_str()).filter(|p| help(p).is_none()).collect();
        assert!(missing.is_empty(), "no help for {missing:?}");
    }

    #[test]
    fn every_entry_is_a_parameter() {
        let reg = crate::Registry::new();
        let keys: Vec<String> = reg.params().iter().map(|p| strip_indices(&p.path)).collect();
        for h in HELP {
            assert!(keys.iter().any(|k| k == h.0), "help for unknown parameter {}", h.0);
            assert!(!h.1.is_empty() && !h.2.is_empty(), "{}", h.0);
        }
    }

    #[test]
    fn indices_are_ignored() {
        assert_eq!(strip_indices("soundboard.mics[1][2]"), "soundboard.mics");
        assert!(help("room.size[2]").is_some());
    }
}
