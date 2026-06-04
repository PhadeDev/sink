use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SinkError;

/// Per-channel output device choices (Phase 4), stored as JSON at
/// `$XDG_CONFIG_HOME/sink/outputs.json`. `None` = follow the system default
/// output (with automatic failover, Sonar-style).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChannelOutputs {
    pub outputs: HashMap<String, Option<String>>,
}

impl ChannelOutputs {
    pub fn config_path() -> Result<PathBuf, SinkError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| SinkError::Config("cannot resolve the user config directory".into()))?;
        Ok(dir.join("sink").join("outputs.json"))
    }

    pub fn load() -> Self {
        let Ok(path) = Self::config_path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|e| {
                eprintln!("sink: ignoring malformed {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), SinkError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            crate::persistence::ensure_private_dir(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SinkError::Config(format!("serialize outputs: {e}")))?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn set(&mut self, sink_name: &str, output: Option<String>) {
        self.outputs.insert(sink_name.to_string(), output);
    }

    pub fn get(&self, sink_name: &str) -> Option<&str> {
        self.outputs.get(sink_name)?.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_with_follow_default_entries() {
        let mut o = ChannelOutputs::default();
        o.set("sink_game", Some("alsa_output.usb-Headset".into()));
        o.set("sink_music", None);
        let json = serde_json::to_string(&o).expect("serializes");
        let back: ChannelOutputs = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, o);
        assert_eq!(back.get("sink_game"), Some("alsa_output.usb-Headset"));
        assert_eq!(back.get("sink_music"), None);
    }
}
