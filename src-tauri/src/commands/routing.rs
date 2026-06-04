use tauri::State;

use crate::audio::types::is_virtual_sink;
use crate::persistence::wireplumber;
use crate::state::AppState;

const MAX_VOLUME: u8 = 150;

/// Move an app stream onto a channel. An empty `sink_name` unassigns the
/// stream (returns it to the system default sink).
///
/// The choice is also recorded as a persistent assignment (Phase 2): saved
/// to `$XDG_CONFIG_HOME/sink/assignments.json`, mirrored to a WirePlumber
/// conf fragment, and re-applied by the stream poll when the app restarts.
#[tauri::command]
pub fn route_app_to_channel(
    state: State<'_, AppState>,
    stream_index: u32,
    sink_name: String,
) -> Result<(), String> {
    if !sink_name.is_empty() && !is_virtual_sink(&sink_name) {
        return Err(format!("unknown channel: {sink_name}"));
    }
    state
        .backend
        .move_stream_to_sink(stream_index, &sink_name)
        .map_err(|e| e.to_string())?;

    // Resolve the stream's identity to record the assignment. The stream is
    // already moved at this point, so persistence failures are reported but
    // the live routing stands.
    let streams = state.backend.list_app_streams().map_err(|e| e.to_string())?;
    let Some(stream) = streams.iter().find(|s| s.index == stream_index) else {
        return Ok(()); // stream vanished between move and lookup
    };

    let assignments = {
        let mut mixer = state.lock_mixer()?;
        if sink_name.is_empty() {
            mixer
                .assignments
                .remove(&stream.match_prop, &stream.match_value);
        } else {
            mixer
                .assignments
                .set(&stream.match_prop, &stream.match_value, &sink_name);
        }
        // The user explicitly placed this stream; don't auto-route it again.
        mixer.auto_routed.insert(stream_index);
        crate::commands::profiles::autosave_active(&mixer);
        mixer.assignments.clone()
    };

    assignments.save().map_err(|e| e.to_string())?;
    wireplumber::write(&assignments).map_err(|e| e.to_string())?;
    Ok(())
}

/// Set a channel's volume (0–150%).
#[tauri::command]
pub fn set_channel_volume(
    state: State<'_, AppState>,
    sink_name: String,
    volume: u8,
) -> Result<(), String> {
    let volume = volume.min(MAX_VOLUME);
    state
        .backend
        .set_sink_volume(&sink_name, volume)
        .map_err(|e| e.to_string())?;

    let mut mixer = state.lock_mixer()?;
    if let Some(channel) = mixer.channel_mut(&sink_name) {
        channel.volume_percent = volume;
    }
    crate::commands::profiles::autosave_active(&mixer);
    Ok(())
}

/// Mute or unmute a channel.
#[tauri::command]
pub fn toggle_channel_mute(
    state: State<'_, AppState>,
    sink_name: String,
    muted: bool,
) -> Result<(), String> {
    state
        .backend
        .set_sink_mute(&sink_name, muted)
        .map_err(|e| e.to_string())?;

    let mut mixer = state.lock_mixer()?;
    if let Some(channel) = mixer.channel_mut(&sink_name) {
        channel.muted = muted;
    }
    crate::commands::profiles::autosave_active(&mixer);
    Ok(())
}

/// Set or clear a persistent display name for an app, keyed by its stream
/// identity. An empty `alias` reverts to the discovered name.
#[tauri::command]
pub fn rename_app(
    state: State<'_, AppState>,
    match_prop: String,
    match_value: String,
    alias: String,
) -> Result<(), String> {
    let aliases = {
        let mut mixer = state.lock_mixer()?;
        mixer.aliases.set(&match_prop, &match_value, &alias);
        mixer.aliases.clone()
    };
    aliases.save().map_err(|e| e.to_string())
}

/// Set the volume of a single app stream (0–150%).
#[tauri::command]
pub fn set_app_volume(
    state: State<'_, AppState>,
    stream_index: u32,
    volume: u8,
) -> Result<(), String> {
    state
        .backend
        .set_app_volume(stream_index, volume.min(MAX_VOLUME))
        .map_err(|e| e.to_string())
}
