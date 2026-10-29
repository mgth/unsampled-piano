//! Plays a flight recording again (piano-play's blackbox): a fresh slot
//! given, block for block, what the player's slot applied ([`EngineSlot::journal`]),
//! with the rebuilt instruments injected where they came in. The output is
//! what the player played, sample for sample: the engines retiring after a
//! rebuild and the output stage (limiter) included.

use std::sync::Arc;

use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

/// What a flight recorder keeps.
pub struct Recording {
    /// Rate, quality, threads and the first instrument (`model`).
    pub cfg: EngineConfig,
    /// (block length, how many in a row).
    pub runs: Vec<(usize, usize)>,
    /// (absolute sample, event), in order.
    pub events: Vec<(u64, Event)>,
    /// (absolute sample, model): an instrument rebuilt from it came in at
    /// the start of the block beginning there.
    pub swaps: Vec<(u64, PianoModel)>,
}

/// The recording's output, left and right.
pub fn replay(rec: &Recording) -> [Vec<f32>; 2] {
    let cfg = rec.cfg;
    let max_block = rec.runs.iter().map(|r| r.0).max().unwrap_or(128);
    let cfg = EngineConfig { max_block: max_block.max(cfg.max_block), ..cfg };
    let (_control, mut slot) = crate::new(cfg.model, cfg);
    let total: usize = rec.runs.iter().map(|(l, c)| l * c).sum();
    let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
    let (mut start, mut next, mut swap) = (0usize, 0usize, 0usize);
    let mut block: Vec<TimedEvent> = Vec::with_capacity(256);
    for &(len, count) in &rec.runs {
        for _ in 0..count {
            // Built as the controller's worker builds (one thread: the slot
            // lends its pool).
            let mut model = None;
            while swap < rec.swaps.len() && rec.swaps[swap].0 as usize <= start {
                model = Some(rec.swaps[swap].1);
                swap += 1;
            }
            if let Some(m) = model {
                let ecfg = EngineConfig { model: m, threads: 1, ..cfg };
                slot.inject(Box::new(Engine::with_instrument(ecfg, Arc::new(ecfg.instrument()))));
            }
            block.clear();
            while next < rec.events.len() && (rec.events[next].0 as usize) < start + len {
                block.push(TimedEvent { frame: (rec.events[next].0 as usize - start) as u32, event: rec.events[next].1 });
                next += 1;
            }
            slot.process_stereo(&block, &mut l[start..start + len], &mut r[start..start + len]);
            start += len;
        }
    }
    [l, r]
}
