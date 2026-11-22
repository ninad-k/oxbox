//! Native tray and global input registration; no browser runtime is created.
use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver},
};
use tray_icon::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
#[derive(Clone, Copy)]
pub enum Event {
    Toggle,
    Show,
    Settings,
    Quit,
    StopMouse,
    CancelSleep,
    StopAwake,
    ToggleAwake,
    Lock,
    ToggleDesktop,
    StopPresentation,
    StopRecording,
}
pub struct Integration {
    manager: Option<GlobalHotKeyManager>,
    active: Option<HotKey>,
    registered: Vec<HotKey>,
    committed: Arc<AtomicU64>,
    tray: Option<TrayIcon>,
    events: Receiver<Event>,
    pub error: Option<String>,
    tray_error: Option<String>,
    shortcut_error: Option<String>,
    recovery: Arc<Mutex<Option<crate::recovery::Handle>>>,
}
impl Integration {
    pub fn new(ctx: &egui::Context, shortcut: &str) -> Self {
        #[cfg(target_os = "linux")]
        let gtk_error = gtk::init().err().map(|error| error.to_string());
        let (sender, events) = mpsc::channel();
        let wake = ctx.clone();
        let keys = sender.clone();
        let committed = Arc::new(AtomicU64::new(0));
        let active_id = committed.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed
                && active_id.load(Ordering::Acquire) == u64::from(event.id) + 1
            {
                let _ = keys.send(Event::Toggle);
                wake.request_repaint();
            }
        }));
        let manager = GlobalHotKeyManager::new();
        let mut shortcut_error = manager.as_ref().err().map(ToString::to_string);
        let manager = manager.ok();
        let mut active = None;
        if let Some(manager) = &manager {
            match shortcut
                .parse::<HotKey>()
                .map_err(|error| error.to_string())
                .and_then(|key| {
                    manager
                        .register(key)
                        .map(|_| key)
                        .map_err(|error| error.to_string())
                }) {
                Ok(key) => active = Some(key),
                Err(message) => {
                    shortcut_error = Some(format!(
                        "Launcher shortcut unavailable: {message}. Choose another in Settings."
                    ))
                }
            }
        }
        let menu_events = sender.clone();
        let wake = ctx.clone();
        let recovery = Arc::new(Mutex::new(None::<crate::recovery::Handle>));
        let recovery_events = recovery.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let event = match event.id.as_ref() {
                "open" => Event::Show,
                "settings" => Event::Settings,
                "quit" => Event::Quit,
                "mouse" => Event::StopMouse,
                "sleep" => Event::CancelSleep,
                "awake" => Event::StopAwake,
                "awake-toggle" => Event::ToggleAwake,
                "lock" => Event::Lock,
                "desktop" => Event::ToggleDesktop,
                "presentation" => Event::StopPresentation,
                "recording" => Event::StopRecording,
                _ => return,
            };
            let action = match event {
                Event::StopMouse => Some(crate::recovery::Action::StopMouse),
                Event::CancelSleep => Some(crate::recovery::Action::CancelSleep),
                Event::StopAwake => Some(crate::recovery::Action::StopAwake),
                Event::StopPresentation => Some(crate::recovery::Action::StopPresentation),
                Event::StopRecording => Some(crate::recovery::Action::StopRecording),
                Event::ToggleAwake => Some(crate::recovery::Action::ToggleAwake),
                Event::Lock => Some(crate::recovery::Action::Lock),
                Event::ToggleDesktop => Some(crate::recovery::Action::ToggleDesktop),
                Event::Quit => Some(crate::recovery::Action::Quit),
                _ => None,
            };
            if let Some(action) = action
                && let Ok(registered) = recovery_events.lock()
                && let Some(handle) = registered.as_ref()
            {
                let _ = handle.request(action);
                if !matches!(event, Event::Quit) {
                    wake.request_repaint();
                    return;
                }
            }
            let _ = menu_events.send(event);
            wake.request_repaint();
        }));
        let wake = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = sender.send(Event::Toggle);
                wake.request_repaint();
            }
        }));
        let tray = (|| -> Result<TrayIcon, Box<dyn std::error::Error>> {
            #[cfg(target_os = "linux")]
            if let Some(error) = gtk_error {
                return Err(std::io::Error::other(error).into());
            }
            let menu = Menu::new();
            let open = MenuItem::with_id("open", "Open Oxbox", true, None);
            let settings = MenuItem::with_id("settings", "Settings", true, None);
            let mouse = MenuItem::with_id("mouse", "Stop Mouse Activity", cfg!(windows), None);
            let sleep = MenuItem::with_id("sleep", "Cancel Sleep Timer", true, None);
            let awake = MenuItem::with_id("awake", "Stop Do Not Sleep", true, None);
            let toggle_awake = MenuItem::with_id("awake-toggle", "Toggle Do Not Sleep", true, None);
            let lock = MenuItem::with_id("lock", "Lock Screen", true, None);
            let desktop = MenuItem::with_id(
                "desktop",
                "Toggle Desktop Icons",
                !cfg!(target_os = "linux"),
                None,
            );
            let presentation = MenuItem::with_id("presentation", "Stop Presentation", true, None);
            let recording = MenuItem::with_id("recording", "Stop Recording", true, None);
            let quit = MenuItem::with_id("quit", "Quit Oxbox", true, None);
            menu.append_items(&[
                &open,
                &settings,
                &PredefinedMenuItem::separator(),
                &toggle_awake,
                &lock,
                &desktop,
                &mouse,
                &sleep,
                &awake,
                &presentation,
                &recording,
                &PredefinedMenuItem::separator(),
                &quit,
            ])?;
            let image =
                image::load_from_memory(include_bytes!("../../../src-tauri/icons/32x32.png"))?
                    .into_rgba8();
            let (w, h) = image.dimensions();
            Ok(TrayIconBuilder::new()
                .with_tooltip("Oxbox · your everyday toolkit")
                .with_icon(tray_icon::Icon::from_rgba(image.into_raw(), w, h)?)
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()?)
        })();
        let tray_error = tray
            .as_ref()
            .err()
            .map(|error| format!("Tray unavailable: {error}. Closing the window will quit Oxbox."));
        committed.store(
            active.map_or(0, |key| u64::from(key.id()) + 1),
            Ordering::Release,
        );
        let mut integration = Self {
            manager,
            active,
            registered: active.into_iter().collect(),
            committed,
            tray: tray.ok(),
            events,
            error: None,
            tray_error,
            shortcut_error,
            recovery,
        };
        integration.refresh_error();
        integration
    }
    pub fn set_recovery(&mut self, handle: crate::recovery::Handle) {
        if let Ok(mut registered) = self.recovery.lock() {
            *registered = Some(handle);
        }
    }
    pub fn poll(&self) -> Vec<Event> {
        #[cfg(target_os = "linux")]
        {
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }
        }
        self.events.try_iter().collect()
    }
    pub fn has_tray(&self) -> bool {
        self.tray.is_some()
    }
    pub fn recording_tooltip(&self, active: bool) {
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(if active {
                "Oxbox · Recording active · use Stop Recording to finish"
            } else {
                "Oxbox · your everyday toolkit"
            }));
        }
    }
    pub fn configure_shortcut(
        &mut self,
        value: &str,
        previous_saved: &str,
        persist: impl FnMut(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        let result = (|| {
            let value = value.trim();
            if value.is_empty() || value.len() > 100 {
                return Err("Use a launcher shortcut between 1 and 100 bytes.".into());
            }
            let key = value.parse::<HotKey>().map_err(|error| error.to_string())?;
            crate::settings_tx::configure_shortcut(self, key, value, previous_saved, persist)
        })();
        self.shortcut_error = result.as_ref().err().cloned();
        self.refresh_error();
        result
    }
    fn refresh_error(&mut self) {
        let errors = [self.tray_error.as_deref(), self.shortcut_error.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        self.error = (!errors.is_empty()).then_some(errors);
    }
}
impl crate::settings_tx::ShortcutBackend<HotKey> for Integration {
    fn active(&self) -> Option<HotKey> {
        self.active
    }
    fn register(&mut self, key: &HotKey) -> Result<(), String> {
        if self.registered.contains(key) {
            return Ok(());
        }
        self.manager
            .as_ref()
            .ok_or("The operating system global shortcut service is unavailable")?
            .register(*key)
            .map_err(|e| e.to_string())?;
        self.registered.push(*key);
        Ok(())
    }
    fn unregister(&mut self, key: &HotKey) -> Result<(), String> {
        self.manager
            .as_ref()
            .ok_or("The operating system global shortcut service is unavailable")?
            .unregister(*key)
            .map_err(|e| e.to_string())?;
        self.registered.retain(|registered| registered != key);
        Ok(())
    }
    fn set_active(&mut self, key: Option<HotKey>) {
        self.active = key;
        self.committed.store(
            key.map_or(0, |key| u64::from(key.id()) + 1),
            Ordering::Release,
        );
    }
}
impl Drop for Integration {
    fn drop(&mut self) {
        if let Ok(mut registered) = self.recovery.lock() {
            *registered = None;
        }
        self.committed.store(0, Ordering::Release);
        if let Some(manager) = &self.manager {
            for key in self.registered.drain(..) {
                let _ = manager.unregister(key);
            }
        }
        GlobalHotKeyEvent::set_event_handler(None::<fn(GlobalHotKeyEvent)>);
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod dependency_tests {
    use gtk::glib::{Variant, variant::ToVariant};

    /// Exercise the actual patched binding without initializing GTK or a desktop.
    /// CI also runs this case under the production optimization profile.
    #[test]
    fn approved_glib_backport_string_iterator_methods() {
        let values = ["first", "日本語", "", "last", "🧰"];
        let variant =
            Variant::array_from_iter::<String>(values.iter().map(|value| value.to_variant()));
        assert_eq!(
            variant.array_iter_str().unwrap().collect::<Vec<_>>(),
            values
        );
        assert_eq!(
            variant.array_iter_str().unwrap().rev().collect::<Vec<_>>(),
            values.iter().rev().copied().collect::<Vec<_>>()
        );
        assert_eq!(variant.array_iter_str().unwrap().last(), Some("🧰"));
        assert_eq!(variant.array_iter_str().unwrap().nth(1), Some("日本語"));
        assert_eq!(variant.array_iter_str().unwrap().nth_back(1), Some("last"));
        let mut mixed = variant.array_iter_str().unwrap();
        assert_eq!(mixed.next(), Some("first"));
        assert_eq!(mixed.next_back(), Some("🧰"));
        assert_eq!(mixed.nth(1), Some(""));
        assert_eq!(mixed.next_back(), Some("last"));
        assert_eq!(mixed.next(), None);
        assert_eq!(mixed.next_back(), None);
        let empty = Variant::array_from_iter::<String>(std::iter::empty());
        assert_eq!(empty.array_iter_str().unwrap().next(), None);
        assert!(42_i32.to_variant().array_iter_str().is_err());
    }
}
