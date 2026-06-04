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

/// Create Sink's config directory (and parents) with owner-only access —
/// routing rules and app history are nobody else's business. Used by every
/// save path that writes under `$XDG_CONFIG_HOME/sink`.
pub fn ensure_private_dir(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

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
