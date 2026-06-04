use serde::{Deserialize, Serialize};

/// True if `sink_name` is one of our managed virtual channels. Channels
/// are user-defined (see persistence::channels) but always carry the
/// `sink_` prefix; Sink's own service nodes are excluded.
pub fn is_virtual_sink(sink_name: &str) -> bool {
    sink_name.starts_with("sink_")
        && !crate::persistence::channels::RESERVED_SINK_NAMES.contains(&sink_name)
}

/// Property values that are useless as names — media frameworks announcing
/// themselves, or placeholder stream titles.
const GENERIC_NAMES: [&str; 13] = [
    "WEBRTC VoiceEngine",
    "audio-src",
    "Playback Stream",
    "playStream",
    "audio stream",
    "Audio Stream",
    "audio player",
    "media player",
    "output",
    "Playback",
    "ALSA Playback",
    "Audio output",
    "Audio Source",
];

/// Runtime/wrapper names that hide the real app — e.g. Spotify is a
/// Chromium shell, so application.name says "Chromium" while the process
/// binary says "spotify". A wrapper beats a generic, but a real name
/// (usually the binary) beats both.
const WRAPPER_NAMES: [&str; 14] = [
    "Chromium",
    "Google Chrome",
    "Chrome",
    "Electron",
    "WINE",
    "wine64-preloader",
    "java",
    "python",
    "python3",
    "node",
    "mono",
    "dotnet",
    "QtWebEngine",
    "CEF",
];

fn name_quality(value: &str) -> u8 {
    if GENERIC_NAMES.iter().any(|g| g.eq_ignore_ascii_case(value)) {
        0
    } else if WRAPPER_NAMES.iter().any(|w| w.eq_ignore_ascii_case(value)) {
        1
    } else {
        2
    }
}

/// Prettify a value for display: lone all-lowercase binary names get a
/// capital ("spotify" → "Spotify"). Identity matching always uses the raw
/// value, so this never affects routing rules.
fn prettify(value: &str) -> String {
    if !value.contains(' ') && value.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        let mut chars = value.chars();
        match chars.next() {
            Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
            None => value.to_string(),
        }
    } else {
        value.to_string()
    }
}

/// Resolve a stream's identity: returns (display name, match property,
/// raw match value). The best-quality candidate along the chain wins:
/// real app names beat runtime wrappers beat generic stream titles.
pub fn resolve_identity(get: impl Fn(&str) -> Option<String>) -> (String, String, String) {
    const CHAIN: [&str; 4] = [
        "application.name",
        "application.process.binary",
        "media.name",
        "node.name",
    ];
    let mut best: Option<(u8, String, String)> = None;
    for key in CHAIN {
        if let Some(value) = get(key) {
            let quality = name_quality(&value);
            // (map_or keeps MSRV 1.77 — Option::is_none_or is 1.82+.)
            if best.as_ref().map_or(true, |(q, _, _)| quality > *q) {
                let stop = quality == 2;
                best = Some((quality, key.to_string(), value));
                if stop {
                    break;
                }
            }
        }
    }
    match best {
        Some((_, key, value)) => (prettify(&value), key, value),
        None => (
            "Unknown".to_string(),
            "application.name".to_string(),
            "Unknown".to_string(),
        ),
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use std::collections::HashMap;

    fn resolve(props: &[(&str, &str)]) -> (String, String, String) {
        let map: HashMap<String, String> = props
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        resolve_identity(|key| map.get(key).cloned())
    }

    #[test]
    fn spotify_masquerading_as_chromium_resolves_via_binary() {
        let (display, prop, value) = resolve(&[
            ("application.name", "Chromium"),
            ("application.process.binary", "spotify"),
            ("media.name", "Playback"),
        ]);
        assert_eq!(display, "Spotify"); // prettified for the UI
        assert_eq!(prop, "application.process.binary");
        assert_eq!(value, "spotify"); // raw for rule matching
    }

    #[test]
    fn discord_webrtc_resolves_via_binary() {
        let (display, prop, _) = resolve(&[
            ("application.name", "WEBRTC VoiceEngine"),
            ("application.process.binary", "Discord"),
        ]);
        assert_eq!(display, "Discord");
        assert_eq!(prop, "application.process.binary");
    }

    #[test]
    fn real_browser_keeps_its_wrapper_name() {
        let (display, _, _) = resolve(&[
            ("application.name", "Chromium"),
            ("application.process.binary", "chromium"),
            ("media.name", "Playback"),
        ]);
        assert_eq!(display, "Chromium"); // wrapper beats generic; no better candidate
    }

    #[test]
    fn firefox_application_name_wins_immediately() {
        let (display, prop, _) = resolve(&[
            ("application.name", "Firefox"),
            ("application.process.binary", "firefox"),
        ]);
        assert_eq!(display, "Firefox");
        assert_eq!(prop, "application.name");
    }

    #[test]
    fn pure_generic_still_shows_something() {
        let (display, _, value) = resolve(&[("media.name", "audio-src"), ("node.name", "audio-src")]);
        assert_eq!(display, "Audio-src");
        assert_eq!(value, "audio-src");
    }
}

/// A running application audio stream (a PulseAudio "sink input").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStream {
    pub index: u32,
    /// Display name (possibly prettified — not for matching).
    pub app_name: String,
    /// PipeWire property the identity was read from (e.g. "application.name").
    pub match_prop: String,
    /// Raw property value; with `match_prop` this is the stream's stable
    /// identity for assignments, aliases and WirePlumber rules.
    pub match_value: String,
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
