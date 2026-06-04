use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SinkError;

/// How Sink's devices are labeled in other apps' device lists.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLabelStyle {
    /// "Game"
    #[default]
    Plain,
    /// "Game (Sink)"
    Suffix,
    /// "Sink · Game"
    Prefix,
}

/// App preferences, stored at `$XDG_CONFIG_HOME/sink/prefs.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub device_label_style: DeviceLabelStyle,
}

impl Prefs {
    pub fn config_path() -> Result<PathBuf, SinkError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| SinkError::Config("cannot resolve the user config directory".into()))?;
        Ok(dir.join("sink").join("prefs.json"))
    }

    pub fn load() -> Self {
        let Ok(path) = Self::config_path() else {
            return Self::default();
        };
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), SinkError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SinkError::Config(format!("serialize prefs: {e}")))?;
        fs::write(&path, json)?;
        Ok(())
    }

    /// Decorate a device label per the chosen style (applied at node
    /// creation; stored labels stay raw).
    pub fn decorate(&self, label: &str) -> String {
        match self.device_label_style {
            DeviceLabelStyle::Plain => label.to_string(),
            DeviceLabelStyle::Suffix => format!("{label} (Sink)"),
            DeviceLabelStyle::Prefix => format!("Sink · {label}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorate_styles() {
        let mut p = Prefs::default();
        assert_eq!(p.decorate("Game"), "Game");
        p.device_label_style = DeviceLabelStyle::Suffix;
        assert_eq!(p.decorate("Game"), "Game (Sink)");
        p.device_label_style = DeviceLabelStyle::Prefix;
        assert_eq!(p.decorate("Game"), "Sink · Game");
    }
}
