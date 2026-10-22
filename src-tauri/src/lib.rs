// ============================================================
// lib.rs  — Tauri application entry point
// ============================================================

pub mod core;
pub mod ipc;

use crate::core::stealth::apply_host_stealth;
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

            // Stealth mode: apply immediately if we detect we're in host mode
            // (In practice, role is determined after user interaction, but we
            //  pre-hide the window and show it only for the controller role)
            // The frontend will call show_window or keep_hidden based on role choice.

            #[cfg(debug_assertions)]
            {
                // In debug builds, open DevTools automatically
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            host_create_session,
            controller_join_session,
            host_apply_answer,
            start_capture,
            stop_capture,
            inject_input,
            get_session,
            apply_stealth,
            show_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Connex application");
}

/// Command: Apply stealth mode (HOST role — exclude from screen capture / Zoom / Meet)
#[tauri::command]
async fn apply_stealth(app: tauri::AppHandle) -> Result<(), String> {
    info!("[cmd] apply_stealth: excluding window from screen capture (Zoom, Google Meet, etc.)");

    app.run_on_main_thread(move || {
        if let Err(e) = apply_host_stealth() {
            log::error!("[stealth] Failed to apply host stealth: {}", e);
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Command: Show the window (CONTROLLER role — show full UI)
#[tauri::command]
async fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    info!("[cmd] show_window");
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}
