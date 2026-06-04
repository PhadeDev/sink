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
///
/// Doubles as the auto-routing enforcement point (Phase 2): the frontend
/// polls this every 2s, and any stream seen for the first time whose app has
/// a saved assignment is moved onto its channel. Each stream is enforced
/// once, so manual re-routing (here or in pavucontrol) isn't fought.
#[tauri::command]
pub fn get_app_streams(state: State<'_, AppState>) -> Result<Vec<AppStream>, String> {
    let mut streams = state.backend.list_app_streams().map_err(|e| e.to_string())?;

    let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    // Only enforce once the virtual sinks exist; otherwise streams would be
    // marked as handled while their target sink can't be moved to yet.
    if mixer.initialized {
        for stream in &mut streams {
            if mixer.auto_routed.contains(&stream.index) {
                continue;
            }
            if let Some(target) = mixer
                .assignments
                .sink_for(&stream.match_prop, &stream.app_name)
            {
                if stream.assigned_sink.as_deref() != Some(target) {
                    match state.backend.move_stream_to_sink(stream.index, target) {
                        Ok(()) => stream.assigned_sink = Some(target.to_string()),
                        Err(e) => eprintln!(
                            "sink: auto-route of {} (#{}) failed: {e}",
                            stream.app_name, stream.index
                        ),
                    }
                }
            }
            mixer.auto_routed.insert(stream.index);
        }
    }

    // Apply user-chosen display names.
    for stream in &mut streams {
        stream.alias = mixer
            .aliases
            .get(&stream.match_prop, &stream.app_name)
            .map(str::to_string);
    }

    Ok(streams)
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

    let (outputs, mic) = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer.init_defaults();
        (mixer.outputs.clone(), mixer.mic.clone())
    };

    // Wire every channel to its saved output (or the system default) so
    // channels are audible from the start.
    for (name, _) in VIRTUAL_SINKS {
        if let Err(e) = state.backend.set_channel_output(name, outputs.get(name)) {
            eprintln!("sink: output routing for {name} failed: {e}");
        }
    }

    // Bring the mic chain up if it was enabled last session.
    if mic.enabled {
        if let Err(e) = state.backend.set_mic_config(&mic) {
            eprintln!("sink: mic chain init failed: {e}");
        }
    }
    Ok(())
}

/// Current per-channel output choices (None = follow system default).
#[tauri::command]
pub fn get_channel_outputs(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, Option<String>>, String> {
    let mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    Ok(VIRTUAL_SINKS
        .iter()
        .map(|(name, _)| {
            (
                (*name).to_string(),
                mixer.outputs.get(name).map(str::to_string),
            )
        })
        .collect())
}

/// Route a channel to an output device; empty `output_name` = follow the
/// system default. Persisted across restarts.
#[tauri::command]
pub fn set_channel_output(
    state: State<'_, AppState>,
    sink_name: String,
    output_name: String,
) -> Result<(), String> {
    let output = if output_name.is_empty() {
        None
    } else {
        Some(output_name)
    };
    state
        .backend
        .set_channel_output(&sink_name, output.as_deref())
        .map_err(|e| e.to_string())?;

    let outputs = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer.outputs.set(&sink_name, output);
        mixer.outputs.clone()
    };
    outputs.save().map_err(|e| e.to_string())
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
