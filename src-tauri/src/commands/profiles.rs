use tauri::State;

use crate::persistence::profiles::{self, Profile, ProfileInfo};
use crate::persistence::wireplumber;
use crate::state::AppState;

const LOCK_ERR: &str = "mixer state lock poisoned";

#[tauri::command]
pub fn list_profiles() -> Result<Vec<ProfileInfo>, String> {
    profiles::list().map_err(|e| e.to_string())
}

/// Bind (or clear, with empty string) an output device that auto-loads
/// this profile when it appears.
#[tauri::command]
pub fn set_profile_trigger(name: String, device: String) -> Result<(), String> {
    let trigger = if device.is_empty() { None } else { Some(device) };
    profiles::set_trigger(&name, trigger).map_err(|e| e.to_string())
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
            outputs: mixer.outputs.clone(),
            // Preserved separately via set_profile_trigger when re-saving.
            trigger_device: None,
        }
    };
    // Re-saving an existing profile keeps its trigger binding.
    let trigger = profiles::load(&profile.name)
        .ok()
        .and_then(|p| p.trigger_device);
    let profile = Profile {
        trigger_device: trigger,
        ..profile
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

    // Apply the profile's output routing per channel.
    for (name, output) in &profile.outputs.outputs {
        if let Err(e) = state.backend.set_channel_output(name, output.as_deref()) {
            eprintln!("sink: profile output routing for {name} failed: {e}");
        }
    }

    let (assignments, outputs) = {
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
        mixer.outputs = profile.outputs.clone();
        mixer.auto_routed.clear();
        (mixer.assignments.clone(), mixer.outputs.clone())
    };

    assignments.save().map_err(|e| e.to_string())?;
    outputs.save().map_err(|e| e.to_string())?;
    wireplumber::write(&assignments).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    profiles::delete(&name).map_err(|e| e.to_string())
}
