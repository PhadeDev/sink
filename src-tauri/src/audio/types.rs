use serde::{Deserialize, Serialize};

/// The four Phase 1 virtual channels: (internal sink name, display label).
/// These exact names are mandated by AGENTS.md.
pub const VIRTUAL_SINKS: [(&str, &str); 4] = [
    ("sink_game", "Game"),
    ("sink_chat", "Chat"),
    ("sink_music", "Music"),
    ("sink_system", "System"),
];

/// Returns the display label for a known virtual sink name.
pub fn label_for(sink_name: &str) -> Option<&'static str> {
    VIRTUAL_SINKS
        .iter()
        .find(|(name, _)| *name == sink_name)
        .map(|(_, label)| *label)
}

/// True if `sink_name` is one of our managed virtual channels.
pub fn is_virtual_sink(sink_name: &str) -> bool {
    label_for(sink_name).is_some()
}

/// A running application audio stream (a PulseAudio "sink input").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStream {
    pub index: u32,
    pub app_name: String,
    /// PipeWire property `app_name` was read from (e.g. "application.name").
    /// Together with `app_name` this is the stream's stable identity, used
    /// for persistent routing assignments and aliases.
    pub match_prop: String,
    /// User-chosen display name overriding `app_name` (set via rename).
    pub alias: Option<String>,
    pub icon_name: Option<String>,
    /// Name of the virtual sink the stream is routed to, if it is one of ours.
    pub assigned_sink: Option<String>,
    pub volume_percent: u8,
    pub muted: bool,
}

/// One of the named virtual channels (Game/Chat/Music/System).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSink {
    /// e.g. "sink_game"
    pub name: String,
    /// e.g. "Game"
    pub label: String,
    pub volume_percent: u8,
    pub muted: bool,
}

/// A physical audio output device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDevice {
    pub index: u32,
    pub name: String,
    pub description: String,
}

/// Phase 3 mic chain configuration (persisted; applied live).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicConfig {
    pub enabled: bool,
    /// node.name of the hardware mic to capture (None = system default).
    pub input_device: Option<String>,
    /// 0–200; 100 = unity.
    pub gain_percent: u8,
    pub gate_enabled: bool,
    pub comp_enabled: bool,
    pub limiter_enabled: bool,
    pub muted: bool,
}

impl Default for MicConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            input_device: None,
            gain_percent: 100,
            gate_enabled: true,
            comp_enabled: true,
            limiter_enabled: true,
            muted: false,
        }
    }
}
