use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::SinkError;

/// Node-name prefix for user-created mixes (the seeded default keeps the
/// historical "sink_stream" name so existing OBS setups keep working).
pub const BUS_PREFIX: &str = "sink_bus_";
pub const DEFAULT_BUS_NODE: &str = "sink_stream";
pub const MAX_BUSES: usize = 4;

/// True if `name` is a mix bus node (not a channel, not a service node).
pub fn is_bus_name(name: &str) -> bool {
    name == DEFAULT_BUS_NODE || name.starts_with(BUS_PREFIX)
}

/// True if `name` is the always-on master mix: it carries every channel,
/// can't be deleted, and its membership is managed automatically.
pub fn is_master(name: &str) -> bool {
    name == DEFAULT_BUS_NODE
}

/// One user-defined mix: a capturable virtual source carrying the chosen
/// channels. The label is what recorders (OBS etc.) display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusDef {
    /// Stable node name, e.g. "sink_stream" or "sink_bus_voice_only".
    pub name: String,
    /// Display label — also the device description recorders see.
    pub label: String,
    /// Channel sink names included in this mix.
    pub channels: Vec<String>,
}

/// The user's mixes, stored at `$XDG_CONFIG_HOME/sink/buses.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Buses {
    pub buses: Vec<BusDef>,
}

impl Default for Buses {
    fn default() -> Self {
        Self {
            buses: vec![BusDef {
                name: DEFAULT_BUS_NODE.to_string(),
                label: "Master Mix".to_string(),
                channels: Vec::new(),
            }],
        }
    }
}

fn slugify(label: &str) -> String {
    let slug: String = label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if slug.is_empty() {
        "mix".to_string()
    } else {
        slug
    }
}

impl Buses {
    pub fn config_path() -> Result<PathBuf, SinkError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| SinkError::Config("cannot resolve the user config directory".into()))?;
        Ok(dir.join("sink").join("buses.json"))
    }

    /// Load from disk. On first run (no file), the default Stream Mix bus
    /// inherits membership from the legacy per-channel `stream_mix` flags.
    pub fn load(legacy_channels: &crate::persistence::channels::Channels) -> Self {
        let path = match Self::config_path() {
            Ok(p) => p,
            Err(_) => return Self::default(),
        };
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<Self>(&raw)
                .ok()
                .unwrap_or_default(),
            Err(_) => {
                let mut buses = Self::default();
                buses.buses[0].channels = legacy_channels
                    .channels
                    .iter()
                    .filter(|c| c.stream_mix)
                    .map(|c| c.name.clone())
                    .collect();
                buses
            }
        }
    }

    pub fn save(&self) -> Result<(), SinkError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SinkError::Config(format!("serialize buses: {e}")))?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&BusDef> {
        self.buses.iter().find(|b| b.name == name)
    }

    /// Ensure the master mix exists, sits first, and carries every channel.
    /// Called wherever the channel set changes (init, add, profile load).
    pub fn sync_master(&mut self, channels: &[String]) {
        let mut def = match self.buses.iter().position(|b| is_master(&b.name)) {
            Some(i) => self.buses.remove(i),
            None => BusDef {
                name: DEFAULT_BUS_NODE.to_string(),
                label: "Master Mix".to_string(),
                channels: Vec::new(),
            },
        };
        def.channels = channels.to_vec();
        self.buses.insert(0, def);
    }

    pub fn add(&mut self, label: &str) -> Result<BusDef, SinkError> {
        let label = label.trim();
        if label.is_empty() || label.len() > 24 {
            return Err(SinkError::Config("mix label must be 1–24 characters".into()));
        }
        // The master mix doesn't count against the user's mixes.
        if self.buses.iter().filter(|b| !is_master(&b.name)).count() >= MAX_BUSES {
            return Err(SinkError::Config(format!(
                "at most {MAX_BUSES} mixes are supported"
            )));
        }
        let base = format!("{BUS_PREFIX}{}", slugify(label));
        let mut name = base.clone();
        let mut counter = 2;
        while self.get(&name).is_some() || name == DEFAULT_BUS_NODE {
            name = format!("{base}_{counter}");
            counter += 1;
        }
        let def = BusDef {
            name,
            label: label.to_string(),
            channels: Vec::new(),
        };
        self.buses.push(def.clone());
        Ok(def)
    }

    pub fn rename(&mut self, name: &str, label: &str) -> Result<(), SinkError> {
        let label = label.trim();
        if label.is_empty() || label.len() > 24 {
            return Err(SinkError::Config("mix label must be 1–24 characters".into()));
        }
        let def = self
            .buses
            .iter_mut()
            .find(|b| b.name == name)
            .ok_or_else(|| SinkError::UnknownSink(name.to_string()))?;
        def.label = label.to_string();
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), SinkError> {
        if is_master(name) {
            return Err(SinkError::Config("the master mix can't be deleted".into()));
        }
        let before = self.buses.len();
        self.buses.retain(|b| b.name != name);
        if self.buses.len() == before {
            return Err(SinkError::UnknownSink(name.to_string()));
        }
        Ok(())
    }

    pub fn set_members(&mut self, name: &str, channels: Vec<String>) -> Result<(), SinkError> {
        if is_master(name) {
            return Err(SinkError::Config(
                "the master mix always carries every channel".into(),
            ));
        }
        let def = self
            .buses
            .iter_mut()
            .find(|b| b.name == name)
            .ok_or_else(|| SinkError::UnknownSink(name.to_string()))?;
        def.channels = channels;
        Ok(())
    }

    /// Drop a deleted channel from every bus's membership.
    pub fn remove_channel(&mut self, channel: &str) {
        for bus in &mut self.buses {
            bus.channels.retain(|c| c != channel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_master_mix() {
        let b = Buses::default();
        assert_eq!(b.buses.len(), 1);
        assert_eq!(b.buses[0].name, "sink_stream");
        assert_eq!(b.buses[0].label, "Master Mix");
        assert!(is_master(&b.buses[0].name));
    }

    #[test]
    fn add_generates_prefixed_unique_names() {
        let mut b = Buses::default();
        let d = b.add("Voice Only").expect("adds");
        assert_eq!(d.name, "sink_bus_voice_only");
        let d2 = b.add("Voice Only").expect("adds dup label");
        assert_eq!(d2.name, "sink_bus_voice_only_2");
        assert!(is_bus_name(&d.name));
        assert!(is_bus_name("sink_stream"));
        assert!(!is_bus_name("sink_game"));
    }

    #[test]
    fn membership_and_channel_removal() {
        let mut b = Buses::default();
        let mix = b.add("Voice Only").expect("adds");
        b.set_members(&mix.name, vec!["sink_game".into(), "sink_chat".into()])
            .expect("sets");
        b.remove_channel("sink_chat");
        assert_eq!(b.get(&mix.name).expect("bus").channels, vec!["sink_game"]);
    }

    #[test]
    fn master_is_protected_and_auto_synced() {
        let mut b = Buses::default();
        assert!(b.remove("sink_stream").is_err());
        assert!(b.set_members("sink_stream", vec!["sink_game".into()]).is_err());
        // Renaming is allowed — recorders see the label.
        b.rename("sink_stream", "Everything").expect("renames");

        b.sync_master(&["sink_game".into(), "sink_chat".into()]);
        let master = b.get("sink_stream").expect("master");
        assert_eq!(master.label, "Everything");
        assert_eq!(master.channels, vec!["sink_game", "sink_chat"]);
        assert_eq!(b.buses[0].name, "sink_stream");

        // Recreated (with the default label) if it ever goes missing.
        b.buses.clear();
        b.sync_master(&["sink_game".into()]);
        assert_eq!(b.buses[0].label, "Master Mix");
        assert_eq!(b.buses[0].channels, vec!["sink_game"]);
    }

    #[test]
    fn master_does_not_count_toward_limit() {
        let mut b = Buses::default();
        for i in 0..MAX_BUSES {
            b.add(&format!("Mix {i}")).expect("adds user mix");
        }
        assert!(b.add("One Too Many").is_err());
        assert_eq!(b.buses.len(), MAX_BUSES + 1); // master + user mixes
    }
}
