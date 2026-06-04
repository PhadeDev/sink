use tauri::State;

use crate::audio::types::{MicConfig, OutputDevice};
use crate::persistence::mic;
use crate::state::AppState;


#[tauri::command]
pub fn get_mic_config(state: State<'_, AppState>) -> Result<MicConfig, String> {
    let mixer = state.lock_mixer()?;
    Ok(mixer.mic.clone())
}

/// Apply and persist the mic chain configuration.
#[tauri::command]
pub fn set_mic_config(state: State<'_, AppState>, config: MicConfig) -> Result<(), String> {
    state
        .backend
        .set_mic_config(&config)
        .map_err(|e| e.to_string())?;
    {
        let mut mixer = state.lock_mixer()?;
        mixer.mic = config.clone();
    }
    mic::save(&config).map_err(|e| e.to_string())
}

/// Hardware microphones available as the chain's input.
#[tauri::command]
pub fn get_input_devices(state: State<'_, AppState>) -> Result<Vec<OutputDevice>, String> {
    state
        .backend
        .list_input_devices()
        .map_err(|e| e.to_string())
}
