use serde::{Deserialize, Serialize};

/// True if `sink_name` is one of our managed virtual channels. Channels
/// are user-defined (see persistence::channels) but always carry the
/// `sink_` prefix; Sink's own service nodes are excluded.
pub fn is_virtual_sink(sink_name: &str) -> bool {
    sink_name.starts_with("sink_")
        && !crate::persistence::channels::RESERVED_SINK_NAMES.contains(&sink_name)
}

/// Property values that are technically present but useless as display
/// names — media frameworks announcing themselves instead of the app.
const GENERIC_NAMES: [&str; 9] = [
    "WEBRTC VoiceEngine",
    "audio-src",
    "Playback Stream",
    "playStream",
    "audio stream",
    "Audio Stream",
    "output",
    "ALSA Playback",
    "Audio output",
];

fn is_generic_name(value: &str) -> bool {
    GENERIC_NAMES.iter().any(|g| g.eq_ignore_ascii_case(value))
}

/// Resolve a stream's display name + identity property. Prefers the first
/// non-generic value along the chain; falls back to the first generic one
/// rather than "Unknown" (a framework name still beats nothing).
pub fn resolve_identity(get: impl Fn(&str) -> Option<String>) -> (String, String) {
    const CHAIN: [&str; 4] = [
        "application.name",
        "application.process.binary",
        "media.name",
        "node.name",
    ];
    let mut fallback: Option<(String, String)> = None;
    for key in CHAIN {
        if let Some(value) = get(key) {
            if !is_generic_name(&value) {
                return (value, key.to_string());
            }
            if fallback.is_none() {
                fallback = Some((value, key.to_string()));
            }
        }
    }
    fallback.unwrap_or_else(|| ("Unknown".to_string(), "application.name".to_string()))
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
    /// True while the stream is actively producing audio (node running /
    /// not corked) — drives the activity indicator in the app list.
    pub active: bool,
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
