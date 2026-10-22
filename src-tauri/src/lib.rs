// ============================================================
// lib.rs  — Tauri application entry point
// ============================================================

pub mod core;
pub mod ipc;

use crate::ipc::commands::*;
use log::info;
use tauri::Manager;

pub struct AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ConnexState::new())
        .setup(|app| {
            info!("[connex] Application starting");

            // Connex runs as a normal, visible application: it shows in the
            // Dock / taskbar and is included in screen shares like any other
            // window. Anyone sharing their screen can see it is running.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();

                // Stealth mode: exclude Connex window from screen capture (Zoom, Google Meet, OBS, Teams)
                let _ = crate::core::stealth::apply_stealth();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_device_identity,
            rotate_passcode,
            apply_stealth,
            host_create_session,
            controller_join_session,
            host_apply_answer,
            start_capture,
            stop_capture,
            inject_input,
            get_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Connex application");
}
