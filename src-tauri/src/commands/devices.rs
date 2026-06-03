use tauri::State;

use crate::audio::types::{AppStream, OutputDevice, VirtualSink, VIRTUAL_SINKS};
use crate::state::AppState;

const LOCK_ERR: &str = "mixer state lock poisoned";

/// Current channel state (volume/mute as tracked by MixerState).
#[tauri::command]
pub fn get_virtual_devices(state: State<'_, AppState>) -> Result<Vec<VirtualSink>, String> {
    let mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    Ok(mixer.channels.clone())
}

/// All running app audio streams.
#[tauri::command]
pub fn get_app_streams(state: State<'_, AppState>) -> Result<Vec<AppStream>, String> {
    state.backend.list_app_streams().map_err(|e| e.to_string())
}

/// Physical output devices (everything that isn't one of our virtual sinks).
#[tauri::command]
pub fn get_output_devices(state: State<'_, AppState>) -> Result<Vec<OutputDevice>, String> {
    state
        .backend
        .list_output_devices()
        .map_err(|e| e.to_string())
}

/// Create the four default virtual sinks and reset them to 100%, unmuted.
/// Idempotent: safe to call again if the sinks already exist.
#[tauri::command]
pub fn init_virtual_devices(state: State<'_, AppState>) -> Result<(), String> {
    for (name, _) in VIRTUAL_SINKS {
        state
            .backend
            .create_virtual_sink(name)
            .map_err(|e| e.to_string())?;
        // Known starting point — adopted sinks from a previous run may carry
        // stale volume/mute.
        state
            .backend
            .set_sink_volume(name, 100)
            .map_err(|e| e.to_string())?;
        state
            .backend
            .set_sink_mute(name, false)
            .map_err(|e| e.to_string())?;
    }

    let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    mixer.init_defaults();
    Ok(())
}

/// Destroy all virtual sinks. Called before the app exits.
#[tauri::command]
pub fn teardown_virtual_devices(state: State<'_, AppState>) -> Result<(), String> {
    let errors = state.teardown_virtual_sinks();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
