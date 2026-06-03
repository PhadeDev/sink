mod audio;
mod commands;
mod error;
mod mixer;
mod persistence;
mod state;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};

use audio::backend::AudioBackend;
use audio::pactl::PactlBackend;
use audio::pw_native::levels::LevelStore;
use audio::pw_native::PipeWireBackend;
use audio::types::VIRTUAL_SINKS;
use state::AppState;

pub fn run() {
    // Prefer the native PipeWire backend (Phase 2); fall back to pactl
    // subprocess calls if the native loop can't come up. Levels (real VU
    // metering) are native-only.
    let (backend, levels): (Arc<dyn AudioBackend>, Option<Arc<LevelStore>>) =
        match PipeWireBackend::new() {
            Ok(backend) => {
                let levels = backend.levels.clone();
                (Arc::new(backend), Some(levels))
            }
            Err(e) => {
                eprintln!("sink: native PipeWire backend unavailable ({e}); using pactl fallback");
                (Arc::new(PactlBackend::new()), None)
            }
        };
    let app_state = AppState::new(backend);

    let result = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::devices::get_virtual_devices,
            commands::devices::get_app_streams,
            commands::devices::get_output_devices,
            commands::devices::init_virtual_devices,
            commands::devices::teardown_virtual_devices,
            commands::routing::route_app_to_channel,
            commands::routing::set_channel_volume,
            commands::routing::toggle_channel_mute,
            commands::routing::set_app_volume,
            commands::routing::rename_app,
            commands::profiles::list_profiles,
            commands::profiles::save_profile,
            commands::profiles::load_profile,
            commands::profiles::delete_profile,
        ])
        .setup(move |app| {
            build_tray(app)?;
            if let Some(levels) = levels {
                spawn_level_emitter(app.handle().clone(), levels);
            }
            Ok(())
        })
        // Close button hides to tray instead of quitting.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(e) = window.hide() {
                    eprintln!("sink: failed to hide window: {e}");
                }
            }
        })
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("sink: fatal error while running tauri application: {e}");
        std::process::exit(1);
    }
}

/// Streams per-channel peak levels to the UI at 10 Hz as `levels` events.
/// Peaks are drained (read-and-reset), so silence decays to zero.
fn spawn_level_emitter(handle: tauri::AppHandle, levels: Arc<LevelStore>) {
    std::thread::spawn(move || loop {
        let payload: HashMap<&'static str, [f32; 2]> = VIRTUAL_SINKS
            .iter()
            .enumerate()
            .map(|(slot, (name, _))| (*name, [levels.drain(slot, 0), levels.drain(slot, 1)]))
            .collect();
        if handle.emit("levels", &payload).is_err() {
            // App is shutting down.
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    });
}

fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start at login",
        true,
        persistence::autostart::is_enabled(),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &autostart, &quit])?;
    let autostart_item = autostart.clone();

    // Dedicated 22px tray glyph from the icon pack (white for the common
    // dark panel; the full-color icon stays on the window/dock).
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-white-22.png"))?;

    TrayIconBuilder::with_id("sink-tray")
        .icon(icon)
        .tooltip("sink")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "autostart" => {
                // Toggle the systemd user unit; reflect the real state back
                // into the menu (the click already flipped the checkbox).
                let result = if persistence::autostart::is_enabled() {
                    persistence::autostart::disable()
                } else {
                    persistence::autostart::enable()
                };
                if let Err(e) = result {
                    eprintln!("sink: autostart toggle failed: {e}");
                }
                let _ = autostart_item.set_checked(persistence::autostart::is_enabled());
            }
            "quit" => {
                // Clean up our virtual sinks before exiting. Best-effort:
                // log failures but never block quitting.
                let state = app.state::<AppState>();
                for err in state.teardown_virtual_sinks() {
                    eprintln!("sink: teardown: {err}");
                }
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
