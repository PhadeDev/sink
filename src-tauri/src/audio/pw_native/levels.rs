use std::sync::atomic::{AtomicU32, Ordering};

use crate::audio::types::VIRTUAL_SINKS;

/// Lock-free per-channel peak store: 4 virtual sinks × stereo, written by
/// the realtime meter capture callbacks, drained by the level emitter.
/// Values are f32 amplitudes bit-cast into AtomicU32.
pub struct LevelStore {
    peaks: [AtomicU32; 8],
}

impl LevelStore {
    pub fn new() -> Self {
        Self {
            peaks: Default::default(),
        }
    }

    pub fn slot_for(sink_name: &str) -> Option<usize> {
        VIRTUAL_SINKS.iter().position(|(name, _)| *name == sink_name)
    }

    /// Raise the stored peak for a sink channel (kept until drained).
    pub fn raise(&self, slot: usize, channel: usize, amplitude: f32) {
        let Some(cell) = self.peaks.get(slot * 2 + channel.min(1)) else {
            return;
        };
        let new = amplitude.to_bits();
        let mut current = cell.load(Ordering::Relaxed);
        while f32::from_bits(current) < amplitude {
            match cell.compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// Read and reset the peak for a sink channel.
    pub fn drain(&self, slot: usize, channel: usize) -> f32 {
        self.peaks
            .get(slot * 2 + channel.min(1))
            .map(|cell| f32::from_bits(cell.swap(0, Ordering::Relaxed)))
            .unwrap_or(0.0)
    }
}

impl Default for LevelStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raise_keeps_maximum_and_drain_resets() {
        let store = LevelStore::new();
        store.raise(0, 0, 0.5);
        store.raise(0, 0, 0.3); // lower — ignored
        assert!((store.drain(0, 0) - 0.5).abs() < f32::EPSILON);
        assert_eq!(store.drain(0, 0), 0.0); // drained
    }

    #[test]
    fn slots_map_virtual_sinks() {
        assert_eq!(LevelStore::slot_for("sink_game"), Some(0));
        assert_eq!(LevelStore::slot_for("sink_system"), Some(3));
        assert_eq!(LevelStore::slot_for("nope"), None);
    }
}
