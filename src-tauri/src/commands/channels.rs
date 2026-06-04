use tauri::State;

use crate::audio::types::VirtualSink;
use crate::persistence::wireplumber;
use crate::state::AppState;

const LOCK_ERR: &str = "mixer state lock poisoned";

/// Create a new channel from a label (sink name is generated). The new
/// channel starts at 100%, unmuted, following the default output.
#[tauri::command]
pub fn add_channel(state: State<'_, AppState>, label: String) -> Result<(), String> {
    let (def, defs) = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        let def = mixer.channel_defs.add(&label).map_err(|e| e.to_string())?;
        (def, mixer.channel_defs.clone())
    };

    if let Err(e) = (|| {
        state.backend.create_virtual_sink(&def.name, &def.label)?;
        state.backend.set_sink_volume(&def.name, 100)?;
        state.backend.set_sink_mute(&def.name, false)?;
        state.backend.set_channel_output(&def.name, None)
    })() {
        // Roll the definition back so config matches reality.
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        let _ = mixer.channel_defs.remove(&def.name);
        return Err(e.to_string());
    }

    defs.save().map_err(|e| e.to_string())?;
    let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
    mixer.channels.push(VirtualSink {
        name: def.name,
        label: def.label,
        volume_percent: 100,
        muted: false,
    });
    Ok(())
}

/// Rename a channel's display label (the sink name stays stable, so
/// assignments, outputs and profiles keep working).
#[tauri::command]
pub fn rename_channel(
    state: State<'_, AppState>,
    sink_name: String,
    label: String,
) -> Result<(), String> {
    let defs = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer
            .channel_defs
            .rename(&sink_name, &label)
            .map_err(|e| e.to_string())?;
        if let Some(channel) = mixer.channel_mut(&sink_name) {
            channel.label = label.trim().to_string();
        }
        mixer.channel_defs.clone()
    };
    defs.save().map_err(|e| e.to_string())
}

/// Delete a channel: streams on it return to the default sink, its
/// assignments are dropped, and the sink is destroyed.
#[tauri::command]
pub fn remove_channel(state: State<'_, AppState>, sink_name: String) -> Result<(), String> {
    // Validate against the definition set first (also enforces "keep one").
    {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer
            .channel_defs
            .remove(&sink_name)
            .map_err(|e| e.to_string())?;
    }

    // Hand the channel's streams back to the default sink before the rug
    // is pulled out from under them.
    if let Ok(streams) = state.backend.list_app_streams() {
        for stream in streams {
            if stream.assigned_sink.as_deref() == Some(sink_name.as_str()) {
                if let Err(e) = state.backend.move_stream_to_sink(stream.index, "") {
                    eprintln!("sink: evacuating {} failed: {e}", stream.app_name);
                }
            }
        }
    }

    state
        .backend
        .destroy_virtual_sink(&sink_name)
        .map_err(|e| e.to_string())?;

    let (defs, assignments, outputs) = {
        let mut mixer = state.mixer.lock().map_err(|_| LOCK_ERR.to_string())?;
        mixer.channels.retain(|c| c.name != sink_name);
        mixer
            .assignments
            .assignments
            .retain(|a| a.sink_name != sink_name);
        mixer.outputs.outputs.remove(&sink_name);
        // Re-evaluate auto-routing with the channel gone.
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
