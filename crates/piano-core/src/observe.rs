//! What a playing engine shows of itself, for the interface's model page
//! ([`crate::Engine::observe`]): every key's state and string energy, the
//! pedals, and the amplitude of the soundboard's lowest modes. Filled on
//! the audio thread without allocating, read a few dozen times a second.

/// Board modes observed: the lowest ones, which carry most of the board's
/// motion (above, the bridge drives the modes too locally to be seen).
pub const OBSERVED_MODES: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyState {
    /// The key is held down.
    pub down: bool,
    /// The hammer is on the strings (the blow is being computed).
    pub hammer: bool,
    /// Damper contact, 0 lifted … 1 resting on the strings (1 for a key
    /// with no damper, whose strings ring freely).
    pub damper: f32,
    /// Energy of the key's strings (their modes' response to the hammer
    /// and to the bridge), arbitrary units, 0 at rest.
    pub energy: f32,
}

impl Default for KeyState {
    fn default() -> Self {
        Self { down: false, hammer: false, damper: 1.0, energy: 0.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    /// Indexed by MIDI note (21–108 are the piano's keys).
    pub keys: [KeyState; 128],
    /// Pedals, 0 up … 1 down.
    pub sustain: f32,
    pub soft: f32,
    pub sostenuto: bool,
    /// Modulus of the complex state of the board's modes, lowest first, in
    /// the order of [`crate::soundboard::Soundboard::new`] (the model's
    /// soundboard with the musician's controls applied); `board_modes` of
    /// them are set (0 while the board sleeps). A mode's velocity at a
    /// point is its state times its shape there.
    pub board: [f32; OBSERVED_MODES],
    pub board_modes: usize,
}

impl Default for Observation {
    fn default() -> Self {
        Self { keys: [KeyState::default(); 128], sustain: 0.0, soft: 0.0, sostenuto: false, board: [0.0; OBSERVED_MODES], board_modes: 0 }
    }
}

impl Observation {
    /// Back to rest, before engines add themselves ([`crate::Engine::observe`]).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Keeps the most of `other`, observed later: what happened between
    /// two looks shows even if it did not last (a blow, a beat's peak).
    /// Keys down or struck at either time, the dampers' highest lift, the
    /// larger energies, pedal positions and board motions.
    pub fn hold(&mut self, other: &Observation) {
        for (k, o) in self.keys.iter_mut().zip(&other.keys) {
            k.down |= o.down;
            k.hammer |= o.hammer;
            k.damper = k.damper.min(o.damper);
            k.energy = k.energy.max(o.energy);
        }
        self.sustain = self.sustain.max(other.sustain);
        self.soft = self.soft.max(other.soft);
        self.sostenuto |= other.sostenuto;
        let n = self.board_modes.max(other.board_modes);
        for (a, b) in self.board[..n].iter_mut().zip(&other.board[..n]) {
            *a = a.max(*b);
        }
        self.board_modes = n;
    }

    /// Adds another engine's view (one finishing its notes after a rebuild):
    /// keys down or struck in either, energies and board motions summed.
    pub fn merge(&mut self, other: &Observation) {
        for (k, o) in self.keys.iter_mut().zip(&other.keys) {
            k.down |= o.down;
            k.hammer |= o.hammer;
            k.damper = k.damper.min(o.damper);
            k.energy += o.energy;
        }
        self.sustain = self.sustain.max(other.sustain);
        self.soft = self.soft.max(other.soft);
        self.sostenuto |= other.sostenuto;
        let n = self.board_modes.max(other.board_modes);
        for (a, b) in self.board[..n].iter_mut().zip(&other.board[..n]) {
            *a = a.hypot(*b);
        }
        self.board_modes = n;
    }
}
