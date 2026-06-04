pub mod active;
pub mod aliases;
pub mod assignments;
pub mod autostart;
pub mod buses;
pub mod channels;
pub mod mic;
pub mod outputs;
pub mod prefs;
pub mod seen;
pub mod profiles;
pub mod wireplumber;

/// Factory reset: delete everything Sink ever saved — the whole config
/// directory (channels, mixes, profiles, assignments, history, prefs)
/// and the WirePlumber routing rules.
pub fn wipe_all() -> Result<(), crate::error::SinkError> {
    if let Some(dir) = dirs::config_dir() {
        let sink_dir = dir.join("sink");
        if sink_dir.exists() {
            std::fs::remove_dir_all(&sink_dir)?;
        }
    }
    if let Ok(conf) = wireplumber::conf_path() {
        if conf.exists() {
            std::fs::remove_file(&conf)?;
        }
    }
    Ok(())
}
