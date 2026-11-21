//! Oxbox desktop shell: tray icon, launcher window and the command bridge to `oxbox-core`.
//!
//! Keep this crate thin. Tool logic belongs in `crates/oxbox-core`; this crate only
//! wires it to the UI (see `commands.rs`) and owns OS-level app behaviour (tray, windows).

mod app_commands;
mod commands;
mod currency_commands;
mod error;
mod file_commands;
mod focus_overlay;
mod jobs;
mod media_commands;
mod mouse_activity_commands;
mod overlay;
mod roadmap_media_commands;
mod roadmap_system_commands;
mod roadmap_time_commands;
mod runtime_privacy;
mod system_commands;
mod tray;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_launcher(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        tray::toggle_launcher(app);
                    }
                })
                .build(),
        )
        .manage(jobs::JobState::default())
        .manage(media_commands::MediaState::default())
        .manage(overlay::AreaCaptureState::default())
        .manage(roadmap_media_commands::RoadmapMediaState::default())
        .manage(mouse_activity_commands::MouseActivityState::default())
        .setup(|app| {
            runtime_privacy::initialize(app)?;
            tray::create(app)?;
            app_commands::initialize(app.handle())?;
            system_commands::initialize(app.handle())?;
            roadmap_time_commands::initialize(app.handle())?;
            roadmap_system_commands::initialize(app.handle());
            media_commands::start_background(app.handle())?;
            if std::env::args().any(|arg| arg == "--autostart")
                && let Some(window) = app.get_webview_window(tray::MAIN_WINDOW)
            {
                let _ = window.hide();
            }
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
            app_commands::shortcut_status,
            app_commands::quit_app,
            runtime_privacy::runtime_privacy_status,
            mouse_activity_commands::mouse_activity_reserve,
            mouse_activity_commands::mouse_activity_start,
            mouse_activity_commands::mouse_activity_stop,
            mouse_activity_commands::mouse_activity_status,
            app_commands::configure_shortcut,
            app_commands::conversion_units,
            app_commands::convert_unit,
            app_commands::time_zones,
            app_commands::world_clocks,
            app_commands::generate_code,
            app_commands::save_code,
            app_commands::copy_code,
            currency_commands::currency_status,
            currency_commands::currency_set_network,
            currency_commands::currency_import,
            currency_commands::currency_refresh,
            currency_commands::currency_convert,
            roadmap_time_commands::dnd_capability,
            roadmap_time_commands::dnd_open_settings,
            roadmap_time_commands::airplane_capability,
            roadmap_time_commands::airplane_open_settings,
            roadmap_time_commands::presentation_status,
            roadmap_time_commands::presentation_configure,
            roadmap_time_commands::presentation_start,
            roadmap_time_commands::presentation_stop,
            roadmap_time_commands::roadmap_time_snapshot,
            roadmap_time_commands::alarm_add,
            roadmap_time_commands::alarm_remove,
            roadmap_time_commands::alarm_snooze,
            roadmap_time_commands::alarm_dismiss,
            roadmap_time_commands::alarm_set_enabled,
            roadmap_time_commands::countdown_add,
            roadmap_time_commands::countdown_remove,
            roadmap_system_commands::roadmap_windows,
            roadmap_system_commands::roadmap_window_top,
            roadmap_system_commands::roadmap_snap,
            roadmap_system_commands::roadmap_show_desktop,
            roadmap_system_commands::roadmap_close_apps,
            roadmap_system_commands::roadmap_displays,
            roadmap_system_commands::roadmap_resolution_status,
            roadmap_system_commands::roadmap_resolution_apply,
            roadmap_system_commands::roadmap_resolution_finish,
            roadmap_system_commands::roadmap_volumes,
            roadmap_system_commands::roadmap_eject,
            roadmap_system_commands::roadmap_power_profiles,
            roadmap_system_commands::roadmap_power_set,
            roadmap_system_commands::roadmap_hidden_files,
            roadmap_system_commands::roadmap_cpu_temperatures,
            roadmap_system_commands::roadmap_launch_sets,
            roadmap_system_commands::roadmap_launch_sets_save,
            roadmap_system_commands::roadmap_launch_set,
            roadmap_system_commands::roadmap_installed_apps,
            roadmap_system_commands::roadmap_uninstall,
            roadmap_system_commands::roadmap_leftovers,
            focus_overlay::roadmap_focus_start,
            focus_overlay::roadmap_focus_stop,
            focus_overlay::roadmap_focus_status,
            focus_overlay::roadmap_focus_frame,
            roadmap_media_commands::roadmap_media_providers,
            roadmap_media_commands::roadmap_microphone_status,
            roadmap_media_commands::roadmap_microphone_set,
            roadmap_media_commands::roadmap_recording_status,
            roadmap_media_commands::roadmap_recording_stop,
            roadmap_media_commands::roadmap_record_audio_start,
            roadmap_media_commands::roadmap_record_screen_start,
            roadmap_media_commands::roadmap_take_video_start,
            roadmap_media_commands::roadmap_convert_media,
            roadmap_media_commands::roadmap_make_gif,
            roadmap_media_commands::roadmap_take_photo,
            roadmap_media_commands::roadmap_recognize_text,
            roadmap_media_commands::roadmap_read_barcode,
            roadmap_media_commands::roadmap_color_sample,
            roadmap_media_commands::roadmap_color_cursor,
            roadmap_media_commands::roadmap_color_targets,
            jobs::cancel_job,
            file_commands::checksum_file_job,
            file_commands::find_duplicates,
            file_commands::trash_files,
            file_commands::create_archive,
            file_commands::extract_archive,
            file_commands::scan_clean_drive,
            file_commands::clean_drive,
            system_commands::system_snapshot,
            system_commands::timer_notification_set,
            system_commands::network_sample,
            system_commands::awake_start,
            system_commands::awake_stop,
            system_commands::awake_status,
            system_commands::lock_screen,
            system_commands::sleep_arm,
            system_commands::sleep_cancel,
            system_commands::sleep_status,
            system_commands::break_configure,
            system_commands::break_status,
            system_commands::break_snooze,
            system_commands::break_skip,
            system_commands::desktop_set_hidden,
            system_commands::desktop_status,
            media_commands::capture_targets,
            media_commands::capture_screenshot,
            media_commands::start_area_capture,
            media_commands::open_screenshot_folder,
            media_commands::clipboard_snapshot,
            media_commands::clipboard_set_paused,
            media_commands::clipboard_pin,
            media_commands::clipboard_delete,
            media_commands::clipboard_clear,
            media_commands::clipboard_copy,
            media_commands::resize_images,
            media_commands::crypt_file,
            overlay::complete_area_capture,
            overlay::cancel_area_capture,
            overlay::area_capture_frame,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Oxbox")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<jobs::JobState>().cancel_all();
                mouse_activity_commands::shutdown(app);
                overlay::shutdown(app);
                media_commands::shutdown(app);
                roadmap_media_commands::shutdown(app);
                roadmap_time_commands::shutdown(app);
                roadmap_system_commands::shutdown(app);
                system_commands::shutdown(app);
            }
        });
}
