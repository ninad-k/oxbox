//! System tray (Windows/Linux) / menu bar (macOS) icon.
//! Left click toggles the launcher; right click opens the menu.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager};

pub const MAIN_WINDOW: &str = "main";

pub fn create(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Oxbox", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let awake = MenuItem::with_id(app, "awake", "Toggle Do Not Sleep", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, "lock", "Lock Screen", true, None::<&str>)?;
    let stop_recording =
        MenuItem::with_id(app, "stop-recording", "Stop Recording", true, None::<&str>)?;
    let cancel_sleep = MenuItem::with_id(
        app,
        "cancel-sleep",
        "Cancel Sleep Timer",
        true,
        None::<&str>,
    )?;
    let desktop = MenuItem::with_id(
        app,
        "desktop",
        "Toggle Desktop Icons",
        cfg!(any(target_os = "windows", target_os = "macos")),
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Oxbox", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &settings,
            &awake,
            &lock,
            &stop_recording,
            &cancel_sleep,
            &desktop,
            &separator,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("oxbox-tray")
        .tooltip("Oxbox")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_launcher(app),
            "settings" => {
                show_launcher(app);
                let _ = app.emit("oxbox://settings", ());
            }
            "awake" => crate::system_commands::toggle_awake(app),
            "lock" => crate::system_commands::lock_now(app),
            "stop-recording" => crate::roadmap_media_commands::stop_from_tray(app),
            "cancel-sleep" => {
                let _ = crate::system_commands::sleep_cancel(app.clone());
            }
            "desktop" => crate::system_commands::toggle_desktop(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_launcher(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub(crate) fn show_launcher(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("oxbox://focus-search", ());
    }
}

pub(crate) fn toggle_launcher(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = window.hide();
    } else {
        show_launcher(app);
    }
}
