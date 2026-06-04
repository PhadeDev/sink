use tauri::State;

use crate::persistence::channels::ChannelDef;
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

/// Apply a saved profile: reconcile the channel **layout** (create missing
/// channels, remove extras — streams evacuate to the default first), then
/// apply volumes/mutes/outputs, replace the assignment set, and clear the
/// auto-route ledger so the new routing is enforced within the next poll.
#[tauri::command]
pub fn load_profile(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let profile = profiles::load(&name).map_err(|e| e.to_string())?;
    if profile.channels.is_empty() {
        return Err(format!("profile {name} has no channels"));
    }

    // ---- layout reconciliation ----
    let current: Vec<ChannelDef> = {
        let mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer.channel_defs.channels.clone()
    };
    for channel in &profile.channels {
        if !current.iter().any(|c| c.name == channel.name) {
            state
                .backend
                .create_virtual_sink(&channel.name, &channel.label)
                .map_err(|e| e.to_string())?;
        }
    }
    for old in &current {
        if !profile.channels.iter().any(|c| c.name == old.name) {
            // Evacuate this channel's streams before destroying it.
            if let Ok(streams) = state.backend.list_app_streams() {
                for stream in streams {
                    if stream.assigned_sink.as_deref() == Some(old.name.as_str()) {
                        let _ = state.backend.move_stream_to_sink(stream.index, "");
                    }
                }
            }
            if let Err(e) = state.backend.destroy_virtual_sink(&old.name) {
                eprintln!("sink: removing {} for profile failed: {e}", old.name);
            }
        }
    }

    // ---- channel state ----
    for channel in &profile.channels {
        state
            .backend
            .set_sink_volume(&channel.name, channel.volume_percent)
            .map_err(|e| e.to_string())?;
        state
            .backend
            .set_sink_mute(&channel.name, channel.muted)
            .map_err(|e| e.to_string())?;
        // Output: profile's choice, or follow-default when unset.
        if let Err(e) = state
            .backend
            .set_channel_output(&channel.name, profile.outputs.get(&channel.name))
        {
            eprintln!("sink: profile output for {} failed: {e}", channel.name);
        }
    }

    let (defs, assignments, outputs) = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer.channel_defs = crate::persistence::channels::Channels {
            channels: profile
                .channels
                .iter()
                .map(|c| ChannelDef {
                    name: c.name.clone(),
                    label: c.label.clone(),
                })
                .collect(),
        };
        mixer.channels = profile.channels.clone();
        mixer.assignments = profile.assignments.clone();
        mixer.outputs = profile.outputs.clone();
        mixer.auto_routed.clear();
        (
            mixer.channel_defs.clone(),
            mixer.assignments.clone(),
            mixer.outputs.clone(),
        )
    };

    defs.save().map_err(|e| e.to_string())?;
    assignments.save().map_err(|e| e.to_string())?;
    outputs.save().map_err(|e| e.to_string())?;
    wireplumber::write(&assignments).map_err(|e| e.to_string())?;
    Ok(())
}

/// Create a profile with a clean slate: the classic four channels at
/// 100%/unmuted, no assignments, all outputs following the default. It is
/// saved but not applied — load it to start fresh.
#[tauri::command]
pub fn create_blank_profile(name: String) -> Result<(), String> {
    let name = profiles::sanitize_name(&name).map_err(|e| e.to_string())?;
    if profiles::load(&name).is_ok() {
        return Err(format!("profile \"{name}\" already exists"));
    }
    let channels = crate::persistence::channels::Channels::default()
        .channels
        .into_iter()
        .map(|def| crate::audio::types::VirtualSink {
            name: def.name,
            label: def.label,
            volume_percent: 100,
            muted: false,
        })
        .collect();
    let profile = Profile {
        name,
        channels,
        assignments: Default::default(),
        outputs: Default::default(),
        trigger_device: None,
    };
    profiles::save(&profile).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    profiles::delete(&name).map_err(|e| e.to_string())
}
