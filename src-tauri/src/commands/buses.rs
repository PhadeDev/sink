use tauri::State;

use crate::persistence::buses::BusDef;
use crate::state::AppState;

/// The user's mixes (buses) with their member channels.
#[tauri::command]
pub fn list_buses(state: State<'_, AppState>) -> Result<Vec<BusDef>, String> {
    let mixer = state.lock_mixer()?;
    Ok(mixer.buses.buses.clone())
}

/// Create a new mix. Recorders see it under `label`. New mixes carry
/// every channel (auto-include) until the user unchecks some.
#[tauri::command]
pub fn add_bus(state: State<'_, AppState>, label: String) -> Result<(), String> {
    let (def, defs, prefs, all) = {
        let mut mixer = state.lock_mixer()?;
        let def = mixer.buses.add(&label).map_err(|e| e.to_string())?;
        (
            def,
            mixer.buses.clone(),
            mixer.prefs.clone(),
            channel_names(&mixer),
        )
    };
    if let Err(e) = state.backend.create_bus(&def.name, &prefs.decorate(&def.label)) {
        let mut mixer = state.lock_mixer()?;
        let _ = mixer.buses.remove(&def.name);
        return Err(e.to_string());
    }
    if let Err(e) = state
        .backend
        .set_bus_members(&def.name, &def.effective_members(&all))
    {
        eprintln!("sink: members for new mix {} failed: {e}", def.name);
    }
    defs.save().map_err(|e| e.to_string())?;
    let mixer = state.lock_mixer()?;
    crate::commands::profiles::autosave_active(&mixer);
    Ok(())
}

/// Rename a mix. The node is recreated so recorders immediately see the
/// new name (the node name stays stable, so OBS configs keep working —
/// capture re-attaches automatically).
#[tauri::command]
pub fn rename_bus(state: State<'_, AppState>, name: String, label: String) -> Result<(), String> {
    let (def, defs, prefs, all) = {
        let mut mixer = state.lock_mixer()?;
        mixer.buses.rename(&name, &label).map_err(|e| e.to_string())?;
        let def = mixer
            .buses
            .get(&name)
            .cloned()
            .ok_or_else(|| "unknown mix".to_string())?;
        (
            def,
            mixer.buses.clone(),
            mixer.prefs.clone(),
            channel_names(&mixer),
        )
    };

    state.backend.destroy_bus(&name).map_err(|e| e.to_string())?;
    state
        .backend
        .create_bus(&def.name, &prefs.decorate(&def.label))
        .map_err(|e| e.to_string())?;
    state
        .backend
        .set_bus_members(&def.name, &def.effective_members(&all))
        .map_err(|e| e.to_string())?;

    defs.save().map_err(|e| e.to_string())?;
    let mixer = state.lock_mixer()?;
    crate::commands::profiles::autosave_active(&mixer);
    Ok(())
}

/// Delete a mix.
#[tauri::command]
pub fn remove_bus(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.backend.destroy_bus(&name).map_err(|e| e.to_string())?;
    let defs = {
        let mut mixer = state.lock_mixer()?;
        mixer.buses.remove(&name).map_err(|e| e.to_string())?;
        crate::commands::profiles::autosave_active(&mixer);
        mixer.buses.clone()
    };
    defs.save().map_err(|e| e.to_string())
}

/// Replace the channel set a mix carries. `channels` is what the user
/// sees checked; for auto-include mixes the complement (the unchecked
/// set) is what gets stored, so future channels keep flowing in.
#[tauri::command]
pub fn set_bus_members(
    state: State<'_, AppState>,
    name: String,
    channels: Vec<String>,
) -> Result<(), String> {
    // Validate against the definition set first, so a rejected request
    // (master mix, unknown name) never reaches the backend — otherwise
    // backend membership and the persisted definition could diverge.
    let stored = {
        let mixer = state.lock_mixer()?;
        if crate::persistence::buses::is_master(&name) {
            return Err("the master mix always carries every channel".to_string());
        }
        let Some(def) = mixer.buses.get(&name) else {
            return Err("unknown mix".to_string());
        };
        if def.exclude {
            channel_names(&mixer)
                .into_iter()
                .filter(|c| !channels.contains(c))
                .collect()
        } else {
            channels.clone()
        }
    };
    state
        .backend
        .set_bus_members(&name, &channels)
        .map_err(|e| e.to_string())?;
    let defs = {
        let mut mixer = state.lock_mixer()?;
        mixer
            .buses
            .set_members(&name, stored)
            .map_err(|e| e.to_string())?;
        crate::commands::profiles::autosave_active(&mixer);
        mixer.buses.clone()
    };
    defs.save().map_err(|e| e.to_string())
}

/// Switch a mix between manual selection and auto-include mode. The
/// carried set is preserved; only what happens to future channels changes.
#[tauri::command]
pub fn set_bus_exclude(
    state: State<'_, AppState>,
    name: String,
    exclude: bool,
) -> Result<(), String> {
    let defs = {
        let mut mixer = state.lock_mixer()?;
        let all = channel_names(&mixer);
        mixer
            .buses
            .set_exclude(&name, exclude, &all)
            .map_err(|e| e.to_string())?;
        crate::commands::profiles::autosave_active(&mixer);
        mixer.buses.clone()
    };
    defs.save().map_err(|e| e.to_string())
}

/// The current channel sink names (the "all channels" set for mixes).
pub(crate) fn channel_names(mixer: &crate::mixer::state::MixerState) -> Vec<String> {
    mixer.channels.iter().map(|c| c.name.clone()).collect()
}
