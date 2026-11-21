//! Oxbox desktop shell: tray icon, launcher window and the command bridge to `oxbox-core`.
//!
//! Keep this crate thin. Tool logic belongs in `crates/oxbox-core`; this crate only
//! wires it to the UI (see `commands.rs`) and owns OS-level app behaviour (tray, windows).

mod commands;
mod error;
mod tray;

use tauri::WindowEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            tray::create(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the launcher hides it to the tray. Quit from the tray menu.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == tray::MAIN_WINDOW
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::transform_text,
            commands::checksum_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Oxbox");
}
