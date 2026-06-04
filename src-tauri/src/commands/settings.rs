use serde::Serialize;
use tauri::State;

use crate::persistence::autostart;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct BackendInfo {
    /// True = native PipeWire backend; false = pactl subprocess fallback.
    pub native: bool,
}

#[tauri::command]
pub fn get_backend_info(state: State<'_, AppState>) -> BackendInfo {
    BackendInfo {
        native: state.backend_native,
    }
}

#[tauri::command]
pub fn get_autostart() -> bool {
    autostart::is_enabled()
}

/// Enable/disable the systemd user unit for autostart on login.
#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<bool, String> {
    let result = if enabled {
        autostart::enable()
    } else {
        autostart::disable()
    };
    result.map_err(|e| e.to_string())?;
    Ok(autostart::is_enabled())
}
