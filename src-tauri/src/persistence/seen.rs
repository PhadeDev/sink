use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SinkError;

/// One app identity Sink has ever observed playing audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeenEntry {
    pub match_prop: String,
    pub match_value: String,
    /// Display name at last sighting (resolver output, pre-alias).
    pub display_name: String,
    pub icon_name: Option<String>,
    /// Unix seconds of the last sighting.
    pub last_seen: u64,
    /// Ignored apps are hidden from the app list and never auto-routed.
    #[serde(default)]
    pub ignored: bool,
}

/// Registry of every app identity ever seen, stored as JSON at
/// `$XDG_CONFIG_HOME/sink/seen_apps.json`. Powers the inactive-apps list
/// and the ignore feature.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeenApps {
    pub apps: Vec<SeenEntry>,
}

impl SeenApps {
    pub fn config_path() -> Result<PathBuf, SinkError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| SinkError::Config("cannot resolve the user config directory".into()))?;
        Ok(dir.join("sink").join("seen_apps.json"))
    }

    pub fn load() -> Self {
        let Ok(path) = Self::config_path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(raw) => {
                let mut seen: Self = serde_json::from_str(&raw).unwrap_or_else(|e| {
                    eprintln!("sink: ignoring malformed {}: {e}", path.display());
                    Self::default()
                });
                // Scrub nameless entries recorded before empty property
                // values were filtered out of identity resolution.
                seen.apps.retain(|a| {
                    !a.display_name.trim().is_empty() && !a.match_value.trim().is_empty()
                });
                seen
            }
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), SinkError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            crate::persistence::ensure_private_dir(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SinkError::Config(format!("serialize seen apps: {e}")))?;
        super::write_atomic(&path, &json)?;
        Ok(())
    }

    fn entry_mut(&mut self, match_prop: &str, match_value: &str) -> Option<&mut SeenEntry> {
        self.apps
            .iter_mut()
            .find(|a| a.match_prop == match_prop && a.match_value == match_value)
    }

    pub fn get(&self, match_prop: &str, match_value: &str) -> Option<&SeenEntry> {
        self.apps
            .iter()
            .find(|a| a.match_prop == match_prop && a.match_value == match_value)
    }

    /// Record a sighting. Returns true when the registry changed in a way
    /// worth persisting (new identity, or display/icon changed) - pure
    /// last_seen bumps return false so the poll doesn't hit the disk.
    pub fn upsert(
        &mut self,
        match_prop: &str,
        match_value: &str,
        display_name: &str,
        icon_name: Option<&str>,
        now: u64,
    ) -> bool {
        if let Some(entry) = self.entry_mut(match_prop, match_value) {
            entry.last_seen = now;
            let changed = entry.display_name != display_name
                || entry.icon_name.as_deref() != icon_name;
            if changed {
                entry.display_name = display_name.to_string();
                entry.icon_name = icon_name.map(str::to_string);
            }
            changed
        } else {
            self.apps.push(SeenEntry {
                match_prop: match_prop.to_string(),
                match_value: match_value.to_string(),
                display_name: display_name.to_string(),
                icon_name: icon_name.map(str::to_string),
                last_seen: now,
                ignored: false,
            });
            true
        }
    }

    pub fn is_ignored(&self, match_prop: &str, match_value: &str) -> bool {
        self.get(match_prop, match_value).is_some_and(|e| e.ignored)
    }

    pub fn set_ignored(&mut self, match_prop: &str, match_value: &str, ignored: bool) -> bool {
        match self.entry_mut(match_prop, match_value) {
            Some(entry) => {
                entry.ignored = ignored;
                true
            }
            None => false,
        }
    }

    pub fn forget(&mut self, match_prop: &str, match_value: &str) {
        self.apps
            .retain(|a| !(a.match_prop == match_prop && a.match_value == match_value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_reports_structural_changes_only() {
        let mut seen = SeenApps::default();
        assert!(seen.upsert("application.name", "Firefox", "Firefox", Some("firefox"), 100));
        // Pure last_seen bump - not worth persisting.
        assert!(!seen.upsert("application.name", "Firefox", "Firefox", Some("firefox"), 200));
        assert_eq!(seen.get("application.name", "Firefox").expect("entry").last_seen, 200);
        // Display change - persist.
        assert!(seen.upsert("application.name", "Firefox", "Firefox ESR", Some("firefox"), 300));
    }

    #[test]
    fn ignore_and_forget() {
        let mut seen = SeenApps::default();
        seen.upsert("media.name", "audio-src", "Audio-src", None, 1);
        assert!(seen.set_ignored("media.name", "audio-src", true));
        assert!(seen.is_ignored("media.name", "audio-src"));
        assert!(!seen.set_ignored("media.name", "nope", true));
        seen.forget("media.name", "audio-src");
        assert!(seen.get("media.name", "audio-src").is_none());
    }
}
