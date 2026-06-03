mod audio;
mod commands;
mod error;
mod mixer;
mod state;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

use audio::pactl::PactlBackend;
use state::AppState;

pub fn run() {
    let app_state = AppState::new(Arc::new(PactlBackend::new()));

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
        ])
        .setup(|app| {
            build_tray(app)?;
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

fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("no default window icon configured")?;

    TrayIconBuilder::with_id("sink-tray")
        .icon(icon)
        .tooltip("sink")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
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
