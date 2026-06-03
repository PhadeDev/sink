use tauri::State;

use crate::persistence::profiles::{self, Profile};
use crate::persistence::wireplumber;
use crate::state::AppState;

const LOCK_ERR: &str = "mixer state lock poisoned";

#[tauri::command]
pub fn list_profiles() -> Result<Vec<String>, String> {
    profiles::list().map_err(|e| e.to_string())
}

/// Snapshot the current mixer state (channels + assignments) under `name`.
#[tauri::command]
pub fn save_profile(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let name = profiles::sanitize_name(&name).map_err(|e| e.to_string())?;
    let profile = {
        let mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        Profile {
            name,
            channels: mixer.channels.clone(),
            assignments: mixer.assignments.clone(),
        }
    };
    profiles::save(&profile).map_err(|e| e.to_string())
}

/// Apply a saved profile: set channel volumes/mutes on the live sinks,
/// replace the assignment set, and clear the auto-route ledger so the new
/// routing is enforced on currently running streams within the next poll.
#[tauri::command]
pub fn load_profile(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let profile = profiles::load(&name).map_err(|e| e.to_string())?;

    // Apply channel state to the live sinks first; bail before touching
    // saved assignments if the audio layer rejects it.
    for channel in &profile.channels {
        state
            .backend
            .set_sink_volume(&channel.name, channel.volume_percent)
            .map_err(|e| e.to_string())?;
        state
            .backend
            .set_sink_mute(&channel.name, channel.muted)
            .map_err(|e| e.to_string())?;
    }

    let assignments = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        // Keep the canonical channel list (defined by VIRTUAL_SINKS) but
        // adopt the profile's volume/mute per channel.
        for channel in &mut mixer.channels {
            if let Some(saved) = profile.channels.iter().find(|c| c.name == channel.name) {
                channel.volume_percent = saved.volume_percent;
                channel.muted = saved.muted;
            }
        }
        mixer.assignments = profile.assignments.clone();
        mixer.auto_routed.clear();
        mixer.assignments.clone()
    };

    assignments.save().map_err(|e| e.to_string())?;
    wireplumber::write(&assignments).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    profiles::delete(&name).map_err(|e| e.to_string())
}
