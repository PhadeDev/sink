use std::collections::HashSet;

use crate::audio::types::{VirtualSink, VIRTUAL_SINKS};
use crate::persistence::aliases::Aliases;
use crate::persistence::assignments::Assignments;

/// In-memory mixer state: the source of truth for channel volume/mute as
/// set through the UI, plus the persistent app→channel assignments.
#[derive(Debug, Default)]
pub struct MixerState {
    pub channels: Vec<VirtualSink>,
    /// True once `init_virtual_devices` has created the sinks.
    pub initialized: bool,
    /// Saved app→channel assignments (persisted to disk + WirePlumber conf).
    pub assignments: Assignments,
    /// User-chosen display names for discovered apps (persisted to disk).
    pub aliases: Aliases,
    /// Stream indices already considered for auto-routing this session.
    /// Each stream is enforced once, on first sight, so a user moving a
    /// stream elsewhere (here or in pavucontrol) isn't fought every poll.
    pub auto_routed: HashSet<u32>,
}

impl MixerState {
    /// Populate the four default channels at 100% volume, unmuted.
    pub fn init_defaults(&mut self) {
        self.channels = VIRTUAL_SINKS
            .iter()
            .map(|(name, label)| VirtualSink {
                name: (*name).to_string(),
                label: (*label).to_string(),
                volume_percent: 100,
                muted: false,
            })
            .collect();
        self.initialized = true;
    }

    pub fn channel_mut(&mut self, sink_name: &str) -> Option<&mut VirtualSink> {
        self.channels.iter_mut().find(|c| c.name == sink_name)
    }

    pub fn reset(&mut self) {
        self.channels.clear();
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_defaults_creates_four_channels() {
        let mut state = MixerState::default();
        state.init_defaults();
        assert_eq!(state.channels.len(), 4);
        assert!(state.initialized);
        assert_eq!(state.channels[0].name, "sink_game");
        assert_eq!(state.channels[0].label, "Game");
        assert!(state.channels.iter().all(|c| c.volume_percent == 100 && !c.muted));
    }

    #[test]
    fn channel_mut_finds_by_name() {
        let mut state = MixerState::default();
        state.init_defaults();
        let chat = state.channel_mut("sink_chat").expect("chat channel exists");
        chat.volume_percent = 85;
        assert_eq!(state.channels[1].volume_percent, 85);
        assert!(state.channel_mut("sink_nope").is_none());
    }
}
