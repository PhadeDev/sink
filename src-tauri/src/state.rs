use std::sync::{Arc, Mutex};

use crate::audio::backend::AudioBackend;
use crate::audio::types::VIRTUAL_SINKS;
use crate::mixer::state::MixerState;

/// Application state managed by Tauri and shared across commands and the tray.
pub struct AppState {
    pub backend: Arc<dyn AudioBackend>,
    pub mixer: Mutex<MixerState>,
}

impl AppState {
    pub fn new(backend: Arc<dyn AudioBackend>) -> Self {
        // Saved assignments are loaded eagerly so auto-routing can enforce
        // them as soon as the sinks exist.
        let mixer = MixerState {
            assignments: crate::persistence::assignments::Assignments::load(),
            aliases: crate::persistence::aliases::Aliases::load(),
            outputs: crate::persistence::outputs::ChannelOutputs::load(),
            ..MixerState::default()
        };
        Self {
            backend,
            mixer: Mutex::new(mixer),
        }
    }

    /// Best-effort teardown of all virtual sinks. Collects error messages
    /// instead of aborting on the first failure so a single bad unload
    /// doesn't leave the remaining sinks behind.
    pub fn teardown_virtual_sinks(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (name, _) in VIRTUAL_SINKS {
            if let Err(e) = self.backend.destroy_virtual_sink(name) {
                errors.push(format!("{name}: {e}"));
            }
        }
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.reset();
        }
        errors
    }
}
