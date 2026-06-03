use tauri::State;

use crate::audio::types::is_virtual_sink;
use crate::state::AppState;

const LOCK_ERR: &str = "mixer state lock poisoned";
const MAX_VOLUME: u8 = 150;

/// Move an app stream onto a channel. An empty `sink_name` unassigns the
/// stream (returns it to the system default sink).
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
        .map_err(|e| e.to_string())
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

    let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    if let Some(channel) = mixer.channel_mut(&sink_name) {
        channel.volume_percent = volume;
    }
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

    let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    if let Some(channel) = mixer.channel_mut(&sink_name) {
        channel.muted = muted;
    }
    Ok(())
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
