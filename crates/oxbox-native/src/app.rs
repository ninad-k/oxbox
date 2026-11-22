use crate::{
    host::{Host, NativeMonitor},
    icons::{self, Icons},
    instance::Instance,
    integration::{Event, Integration},
    recovery::{Action as RecoveryAction, Failure as RecoveryFailure, RecoveryWorker},
    tools::{
        maintenance::MaintenanceTools, media::MediaTools, system::SystemTools,
        utilities::UtilitiesTools,
    },
};
use eframe::egui::{self, Color32, RichText};
use oxbox_core::launcher::{Catalog, Preferences, Theme};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
pub struct App {
    host: Host,
    catalog: Catalog,
    icons: Icons,
    prefs: Preferences,
    persisted: Preferences,
    view: String,
    query: String,
    active: Option<String>,
    settings: bool,
    error: Option<String>,
    storage_ready: bool,
    shortcut: String,
    login: bool,
    integration: Integration,
    system: SystemTools,
    media: MediaTools,
    utilities: UtilitiesTools,
    maintenance: MaintenanceTools,
    quitting: bool,
    activation: Arc<AtomicBool>,
    heartbeat_stop: Arc<AtomicBool>,
    heartbeat: Option<JoinHandle<()>>,
    _instance: Instance,
    painted_theme: Option<egui::Theme>,
    recording_active: bool,
    monitor_refresh: std::time::Instant,
    job_registry: crate::jobs::Registry,
    recovery: Option<RecoveryWorker>,
    presets: crate::preset_ui::PresetUi,
    starter_selected: Vec<String>,
    starter_hidden: bool,
}
impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mut host: Host,
        instance: Instance,
        autostart: bool,
        default_startup_allowed: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let job_registry = crate::jobs::register_context(&cc.egui_ctx);
        let catalog = Catalog::bundled()?;
        let icons = Icons::bundled()?;
        // Use the host's UI font without redistributing proprietary font files.
        // Bundled fonts and eframe's local system fallback remain available.
        #[cfg(target_os = "windows")]
        if let Some(directory) = std::env::var_os("WINDIR") {
            let path = std::path::PathBuf::from(directory).join("Fonts/segoeui.ttf");
            if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() <= 8 * 1024 * 1024)
                && let Ok(bytes) = std::fs::read(path)
            {
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "host-ui".into(),
                    Arc::new(egui::FontData::from_owned(bytes)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "host-ui".into());
                cc.egui_ctx.set_fonts(fonts);
            }
        }
        let boot = crate::startup::initialize(&host, default_startup_allowed);
        let login = boot.enabled;
        let loaded = host.load_key::<Preferences>("native-shell.json", "preferences");
        let (mut prefs, mut error) = match loaded {
            Ok(Some(prefs)) => (prefs, None),
            Ok(None) => match host.load::<serde_json::Value>("settings.json") {
                Ok(legacy) => (Preferences::from_legacy(&legacy.unwrap_or_default()), None),
                Err(error) => (Preferences::default(), Some(error.to_string())),
            },
            Err(error) => (Preferences::default(), Some(error.to_string())),
        };
        let storage_ready = error.is_none();
        if let Some(startup_error) = boot.error {
            error = Some(match error {
                Some(previous) => format!("{previous}\n{startup_error}"),
                None => startup_error,
            });
        }
        prefs.sanitize();
        if storage_ready {
            host.save_key("native-shell.json", "preferences", &prefs)?;
        }
        cc.egui_ctx.set_theme(match prefs.theme {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        });
        if let Some(window) = cc.winit_window() {
            crate::host::dialogs::attach_parent(&cc.egui_ctx, window.clone());
            host.monitors = window
                .available_monitors()
                .enumerate()
                .map(|(index, monitor)| {
                    let p = monitor.position();
                    let s = monitor.size();
                    NativeMonitor {
                        index,
                        name: monitor
                            .name()
                            .unwrap_or_else(|| format!("Display {}", index + 1)),
                        x: p.x,
                        y: p.y,
                        width: s.width,
                        height: s.height,
                        scale: monitor.scale_factor(),
                    }
                })
                .collect();
            if let Ok(handle) = window.window_handle() {
                host.main_window_id = native_capture_id(handle.as_raw());
            }
        }
        let integration = Integration::new(&cc.egui_ctx, &prefs.shortcut);
        let shortcut = prefs.shortcut.clone();
        let activation = Arc::new(AtomicBool::new(false));
        let heartbeat_stop = Arc::new(AtomicBool::new(false));
        let heartbeat = Some(instance.watch(
            cc.egui_ctx.clone(),
            activation.clone(),
            heartbeat_stop.clone(),
        )?);
        let presets = crate::preset_ui::PresetUi::load(&host);
        let mut app = Self {
            host,
            catalog,
            icons,
            persisted: prefs.clone(),
            prefs,
            view: "favorites".into(),
            query: String::new(),
            active: None,
            settings: false,
            error,
            storage_ready,
            shortcut,
            login,
            integration,
            system: SystemTools::default(),
            media: MediaTools::default(),
            utilities: UtilitiesTools::default(),
            maintenance: MaintenanceTools::default(),
            quitting: false,
            activation,
            heartbeat_stop,
            heartbeat,
            _instance: instance,
            painted_theme: None,
            recording_active: false,
            monitor_refresh: std::time::Instant::now(),
            job_registry,
            presets,
            starter_selected: vec![
                "unit-converter".into(),
                "timer-stopwatch".into(),
                "screenshot".into(),
            ],
            starter_hidden: false,
            recovery: None,
        };
        app.system.initialize(&mut app.host, &cc.egui_ctx);
        app.utilities.tick(&mut app.host);
        app.media.tick(&mut app.host);
        let system = app.system.recovery_handle();
        let media = app.media.recovery_handle();
        let registry = app.job_registry.clone();
        let time_stop = app.utilities.shutdown_request();
        let notifier = app.host.notifier.clone();
        let prepare_system = system.clone();
        let prepare_media = media.clone();
        let prepare_notifier = notifier.clone();
        let wake = cc.egui_ctx.clone();
        let recovery = RecoveryWorker::new(
            move |action| match action {
                RecoveryAction::Quit => {
                    let generation = prepare_system.request_shutdown();
                    registry.close();
                    time_stop.store(true, Ordering::Release);
                    prepare_notifier.request_stop();
                    prepare_media.request_shutdown();
                    generation
                }
                RecoveryAction::StopRecording => prepare_media.invalidate_pending(),
                _ => prepare_system.prepare(action),
            },
            move |action, generation| {
                let result = match action {
                    RecoveryAction::StopRecording => media.stop_recording(),
                    RecoveryAction::Quit => system.shutdown(),
                    RecoveryAction::QuitMedia => media.shutdown(),
                    _ => system.execute(action, generation),
                };
                result.map_err(|error| match error {
                    oxbox_core::CoreError::Cancelled => RecoveryFailure::Cancelled,
                    other => RecoveryFailure::Error(other.to_string()),
                })
            },
            move || wake.request_repaint(),
            move |error| notifier.notify("Recovery action failed", error),
        )?;
        app.integration.set_recovery(recovery.handle());
        app.recovery = Some(recovery);
        if autostart && app.integration.has_tray() {
            app.host.hide(&cc.egui_ctx);
        } else if autostart {
            let message = "Oxbox started visibly because the tray could not be created. Login launch remains configurable in Settings.";
            app.error = Some(match app.error.take() {
                Some(previous) => format!("{previous}\n{message}"),
                None => message.into(),
            });
        }
        Ok(app)
    }
    fn save(&mut self) -> Result<(), String> {
        if !self.storage_ready {
            self.prefs = self.persisted.clone();
            return Err(
                "Preferences were not loaded. Back up and repair the original file before editing."
                    .into(),
            );
        }
        if let Err(error) = self
            .host
            .save_key("native-shell.json", "preferences", &self.prefs)
        {
            self.prefs = self.persisted.clone();
            return Err(error.to_string());
        }
        self.persisted = self.prefs.clone();
        Ok(())
    }
    fn leave_active(&mut self) {
        if let Some(id) = &self.active {
            self.utilities.leave_tool(id);
            self.media.leave_tool(id);
            self.system.leave_tool(id);
            self.maintenance.leave_tool(id);
        }
        self.utilities.deactivate();
    }
    fn select(&mut self, view: &str) {
        self.leave_active();
        self.view = view.into();
        self.query.clear();
        self.active = None;
        self.settings = false;
    }
    fn show(&mut self, ctx: &egui::Context) {
        self.reveal(ctx, RevealIntent::Restore);
    }
    fn focus_search(&mut self, ctx: &egui::Context) {
        self.reveal(ctx, RevealIntent::Search);
    }
    fn reveal(&mut self, ctx: &egui::Context, intent: RevealIntent) {
        let policy = reveal_policy(intent, self.active.is_some(), self.settings);
        if policy.reset_to_launcher {
            self.leave_active();
            self.settings = false;
            self.active = None;
        }
        self.host.restore(ctx);
        if policy.focus_search {
            ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("search-tools")));
        }
    }
    fn shutdown(&mut self) {
        self.system.recovery_handle().request_shutdown();
        self.job_registry.close();
        self.utilities
            .shutdown_request()
            .store(true, Ordering::Release);
        self.host.notifier.request_stop();
        self.media.recovery_handle().request_shutdown();
        if let Some(mut recovery) = self.recovery.take() {
            // Normal UI Quit uses the same parallel cleanup lanes as tray Quit.
            // Closing the window must not serialize recording finalization
            // behind a potentially blocked system operation.
            let _ = recovery.handle().request(RecoveryAction::Quit);
            recovery.shutdown();
        }
        self.utilities.deactivate();
        self.system.shutdown();
        self.media.shutdown();
        self.utilities.shutdown();
        self.maintenance.shutdown();
        self.heartbeat_stop.store(true, Ordering::Release);
        if let Some(heartbeat) = self.heartbeat.take() {
            heartbeat.thread().unpark();
            let _ = heartbeat.join();
        }
        self.host.shutdown();
    }
    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        self.shutdown();
        crate::host::dialogs::detach_parent(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn theme(&mut self, ctx: &egui::Context) {
        let theme = ctx.theme();
        if self.painted_theme == Some(theme) {
            return;
        }
        self.painted_theme = Some(theme);
        let dark = theme == egui::Theme::Dark;
        let mut style = egui::Style {
            visuals: if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            },
            ..Default::default()
        };
        style.visuals.panel_fill = if dark {
            Color32::from_rgb(17, 23, 32)
        } else {
            Color32::from_rgb(245, 247, 250)
        };
        style.visuals.window_fill = style.visuals.panel_fill;
        style.visuals.selection.bg_fill = if dark {
            Color32::from_rgb(63, 40, 29)
        } else {
            Color32::from_rgb(255, 234, 219)
        };
        style.visuals.selection.stroke = egui::Stroke::new(
            1.0,
            if dark {
                Color32::from_rgb(255, 157, 96)
            } else {
                Color32::from_rgb(155, 59, 20)
            },
        );
        style.visuals.override_text_color = Some(if dark {
            Color32::from_rgb(231, 237, 245)
        } else {
            Color32::from_rgb(26, 35, 47)
        });
        style.visuals.extreme_bg_color = if dark {
            Color32::from_rgb(13, 19, 27)
        } else {
            Color32::WHITE
        };
        style.visuals.widgets.inactive.bg_fill = if dark {
            Color32::from_rgb(24, 33, 45)
        } else {
            Color32::WHITE
        };
        style.visuals.widgets.inactive.weak_bg_fill = style.visuals.widgets.inactive.bg_fill;
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, crate::accessibility::control_border(dark));
        style.visuals.widgets.hovered.bg_fill = if dark {
            Color32::from_rgb(37, 48, 64)
        } else {
            Color32::from_rgb(232, 238, 246)
        };
        style.visuals.widgets.hovered.weak_bg_fill = style.visuals.widgets.hovered.bg_fill;
        style.visuals.widgets.hovered.bg_stroke = style.visuals.selection.stroke;
        style.visuals.widgets.active.bg_fill = style.visuals.selection.bg_fill;
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
        }
        style.spacing.item_spacing = egui::vec2(10.0, 12.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.spacing.interact_size.y = 32.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(24.0));
        ctx.set_global_style(style);
    }
    fn settings_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();
        let appearance_label = ui.label("Appearance");
        let old = self.prefs.theme;
        let appearance = egui::ComboBox::from_id_salt("appearance")
            .selected_text(format!("{:?}", old))
            .show_ui(ui, |ui| {
                for (theme, name) in [
                    (Theme::System, "System"),
                    (Theme::Light, "Light"),
                    (Theme::Dark, "Dark"),
                ] {
                    ui.selectable_value(&mut self.prefs.theme, theme, name);
                }
            })
            .response
            .labelled_by(appearance_label.id);
        crate::accessibility::combo_expansion(&appearance);
        if old != self.prefs.theme {
            ui.ctx().set_theme(match self.prefs.theme {
                Theme::System => egui::ThemePreference::System,
                Theme::Light => egui::ThemePreference::Light,
                Theme::Dark => egui::ThemePreference::Dark,
            });
            if let Err(error) = self.save() {
                self.error = Some(error);
                ui.ctx().set_theme(match self.prefs.theme {
                    Theme::System => egui::ThemePreference::System,
                    Theme::Light => egui::ThemePreference::Light,
                    Theme::Dark => egui::ThemePreference::Dark,
                });
            }
        }
        let was = self.login;
        if ui
            .add_enabled(
                crate::startup::settings_available(),
                egui::Checkbox::new(&mut self.login, "Launch Oxbox at login"),
            )
            .on_disabled_hover_text(
                "Login startup is unavailable in development and isolated QA profiles.",
            )
            .changed()
        {
            let result = crate::startup::set_enabled(&self.host, self.login);
            match result {
                Ok(enabled) => self.login = enabled,
                Err(error) => {
                    self.login = was;
                    self.error = Some(error.to_string());
                }
            }
        }
        let label = ui.label("Global launcher shortcut");
        ui.text_edit_singleline(&mut self.shortcut)
            .labelled_by(label.id);
        ui.label("For example CommandOrControl+Alt+O. Existing custom shortcuts are preserved.");
        if ui.button("Save shortcut").clicked() {
            let previous = self.prefs.clone();
            let value = self.shortcut.trim().to_owned();
            let host = &self.host;
            let storage_ready = self.storage_ready;
            match self
                .integration
                .configure_shortcut(&value, &previous.shortcut, |binding| {
                    if !storage_ready {
                        return Err("Preference storage is not available".into());
                    }
                    let mut candidate = previous.clone();
                    candidate.shortcut = binding.into();
                    host.save_key("native-shell.json", "preferences", &candidate)
                        .map_err(|error| error.to_string())
                }) {
                Ok(()) => {
                    self.prefs.shortcut = value;
                    self.persisted = self.prefs.clone();
                }
                Err(error) => self.error = Some(error),
            }
        }
        ui.separator();
        ui.label("Native renderer · eframe 0.37 · no embedded browser");
        ui.label("Settings and favorites stay on this computer. Clipboard history starts paused and remains in memory. Currency networking requires explicit opt-in and a manual refresh.");
        if ui.button("Open local data folder").clicked()
            && let Err(error) = open::that(self.host.data_dir())
        {
            self.error = Some(error.to_string());
        }
    }
    fn browse(&mut self, ui: &mut egui::Ui) {
        let search = crate::accessibility::search(ui, &mut self.query);
        let enter = (search.has_focus() || search.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Enter));
        match oxbox_core::quick_actions::parse(&self.query) {
            Ok(Some(action)) => {
                use oxbox_core::quick_actions::QuickAction;
                ui.add_space(12.0);
                ui.heading("Quick action");
                match &action {
                    QuickAction::Calculate { result, .. } => {
                        ui.heading(result.to_string());
                        if ui.button("Copy calculated result").clicked() || enter {
                            ui.ctx().copy_text(result.to_string());
                            crate::accessibility::status(ui, "Result copied.");
                        }
                    }
                    QuickAction::Convert {
                        value,
                        from,
                        to,
                        result,
                    } => {
                        ui.heading(format!("{value} {from} = {result} {to}"));
                        if ui.button("Copy conversion result").clicked() {
                            ui.ctx().copy_text(result.to_string());
                        }
                        if ui.button("Open configured Unit Converter").clicked() || enter {
                            match self.utilities.configure_quick_action(&action) {
                                Ok(()) => {
                                    self.active = Some("unit-converter".into());
                                    self.query.clear();
                                }
                                Err(error) => self.error = Some(error.to_string()),
                            }
                        }
                    }
                    QuickAction::Timer { seconds } => {
                        ui.heading(format!("Timer · {seconds} seconds"));
                        ui.label("Opens a configured timer. Start it explicitly from the tool.");
                        if ui.button("Open configured timer").clicked() || enter {
                            match self.utilities.configure_quick_action(&action) {
                                Ok(()) => {
                                    self.active = Some("timer-stopwatch".into());
                                    self.query.clear();
                                }
                                Err(error) => self.error = Some(error.to_string()),
                            }
                        }
                    }
                }
                ui.weak("Local actions only. No shell commands or maintenance changes run from this input.");
                return;
            }
            Err(error) => {
                crate::accessibility::alert(ui, error);
            }
            Ok(None) => {}
        }
        let result_ids: Vec<String> = self
            .catalog
            .search(&self.view, &self.query, &self.prefs.favorites)
            .into_iter()
            .map(|tool| tool.id.clone())
            .collect();
        if enter {
            self.active = result_ids.first().cloned();
        }
        ui.add_space(16.0);
        let searching = !self.query.trim().is_empty();
        ui.heading(if searching {
            "Search results"
        } else if self.view == "favorites" {
            "Dashboard"
        } else if self.view == "all" {
            "Tool library"
        } else {
            self.catalog
                .categories
                .iter()
                .find(|category| category.id == self.view)
                .map(|category| category.label.as_str())
                .unwrap_or("Tools")
        });
        ui.label(if self.view == "favorites" && !searching {
            "Just the tools you choose. Always within reach.".into()
        } else {
            format!(
                "{} {} for the everyday details.",
                result_ids.len(),
                if result_ids.len() == 1 {
                    "tool"
                } else {
                    "tools"
                }
            )
        });
        ui.add_space(12.0);
        if result_ids.is_empty() {
            ui.add_space(40.0);
            ui.heading(if searching {
                "No tools match your search"
            } else {
                "Make room for your essentials"
            });
            ui.label(if searching {
                "Try a tool name or category."
            } else {
                "Star a tool in the library to add it to your dashboard."
            });
            if !searching && ui.button("Explore the tools").clicked() {
                self.select("all");
            }
            if !searching && self.view == "favorites" && !self.starter_hidden {
                ui.add_space(16.0);
                ui.label("Choose a few favorites to get started. Nothing starts automatically.");
                for (id, label) in [
                    ("unit-converter", "Unit Converter"),
                    ("timer-stopwatch", "Timer & Stopwatch"),
                    ("screenshot", "Screenshot"),
                    ("resize-images", "Resize Images"),
                    ("verify-checksum", "Verify Checksum"),
                ] {
                    let mut selected = self.starter_selected.iter().any(|item| item == id);
                    if ui.checkbox(&mut selected, label).changed() {
                        if selected {
                            self.starter_selected.push(id.into());
                        } else {
                            self.starter_selected.retain(|item| item != id);
                        }
                    }
                }
                if ui
                    .add_enabled(
                        !self.starter_selected.is_empty(),
                        egui::Button::new("Pin selected favorites"),
                    )
                    .clicked()
                {
                    self.prefs.favorites = self.starter_selected.clone();
                    if let Err(error) = self.save() {
                        self.error = Some(error);
                    }
                }
                if ui.button("Keep dashboard empty").clicked() {
                    self.starter_hidden = true;
                }
            }
            if !searching {
                ui.add_space(12.0);
                ui.weak(format!("Open from anywhere: {}", self.prefs.shortcut));
                ui.label("Closing this window keeps Oxbox in the tray. Ctrl/Cmd Q or Quit Oxbox stops it.");
                ui.weak("Try quick actions: = 2 + 3 · 12 inches to cm · timer 10m");
            }
            return;
        }
        let columns = if ui.available_width() >= 470.0 { 2 } else { 1 };
        let width = (ui.available_width() - 12.0 * (columns - 1) as f32) / columns as f32;
        for (row, ids) in result_ids.chunks(columns).enumerate() {
            ui.columns(columns, |cells| {
                for (column, id) in ids.iter().enumerate() {
                    let index = row * columns + column;
                    let ui = &mut cells[column];
                    let Some(tool) = self
                        .catalog
                        .tools
                        .iter()
                        .find(|tool| &tool.id == id)
                        .cloned()
                    else {
                        continue;
                    };
                    egui::Frame::group(ui.style())
                        .fill(ui.visuals().widgets.inactive.bg_fill)
                        .corner_radius(10)
                        .inner_margin(16)
                        .show(ui, |ui| {
                            ui.set_width((width - 34.0).max(100.0));
                            ui.set_min_height(172.0);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(26.0, 26.0),
                                    egui::Sense::hover(),
                                );
                                self.icons.draw(
                                    ui,
                                    &tool.icon,
                                    rect,
                                    ui.visuals().selection.stroke.color,
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let favorite = self.prefs.favorites.contains(id);
                                        if icons::pin(ui, favorite, &tool.name).clicked() {
                                            if favorite {
                                                self.prefs.favorites.retain(|value| value != id);
                                            } else {
                                                self.prefs.favorites.push(id.clone());
                                            }
                                            if let Err(error) = self.save() {
                                                self.error = Some(error);
                                            }
                                        }
                                    },
                                );
                            });
                            if ui
                                .add_sized(
                                    [ui.available_width(), 30.0],
                                    egui::Button::new(RichText::new(&tool.name).strong())
                                        .frame(false),
                                )
                                .clicked()
                            {
                                self.active = Some(id.clone());
                            }
                            ui.add(egui::Label::new(&tool.description).wrap());
                            ui.add_space(8.0);
                            ui.weak(
                                self.catalog
                                    .categories
                                    .iter()
                                    .find(|category| category.id == tool.category)
                                    .map(|category| category.label.as_str())
                                    .unwrap_or("Tool"),
                            );
                            if self.view == "favorites" && !searching && index < 9 {
                                ui.weak(format!("Alt {}", index + 1));
                            }
                        });
                }
            });
            ui.add_space(12.0);
        }
        if self.view == "favorites" && !searching {
            ui.add_space(24.0);
            ui.weak("Ctrl/Cmd K search  ·  Alt 1–9 favorites");
            ui.label(format!("Open Oxbox from anywhere: {}", self.prefs.shortcut));
        }
    }
}
impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.quitting {
            return;
        }
        if let Some(window) = frame.winit_window() {
            crate::host::dialogs::attach_parent(ctx, window.clone());
        }
        self.theme(ctx);
        if let Some(recovery) = &mut self.recovery {
            for error in recovery.drain_errors() {
                self.error = Some(error);
            }
        }
        if self.monitor_refresh.elapsed() >= std::time::Duration::from_secs(1) {
            self.monitor_refresh = std::time::Instant::now();
            if let Some(window) = frame.winit_window() {
                self.host.monitors = window
                    .available_monitors()
                    .enumerate()
                    .map(|(index, monitor)| {
                        let p = monitor.position();
                        let s = monitor.size();
                        NativeMonitor {
                            index,
                            name: monitor
                                .name()
                                .unwrap_or_else(|| format!("Display {}", index + 1)),
                            x: p.x,
                            y: p.y,
                            width: s.width,
                            height: s.height,
                            scale: monitor.scale_factor(),
                        }
                    })
                    .collect();
            }
        }
        self.system.tick(&mut self.host);
        self.media.tick(&mut self.host);
        self.utilities.tick(&mut self.host);
        self.maintenance.tick();
        if let Ok(status) = self.media.recording_status()
            && status.active != self.recording_active
        {
            self.recording_active = status.active;
            self.integration.recording_tooltip(status.active);
        }
        for error in self.host.notification_errors() {
            self.error = Some(error);
        }
        if self.activation.swap(false, Ordering::AcqRel) {
            self.show(ctx);
        }
        for event in self.integration.poll() {
            match event {
                Event::Toggle => {
                    if self.host.visible.load(Ordering::Acquire)
                        && ctx.input(|input| input.viewport().focused.unwrap_or(false))
                    {
                        self.utilities.deactivate();
                        self.maintenance.deactivate();
                        self.host.hide(ctx);
                    } else {
                        self.show(ctx);
                    }
                }
                Event::Show => self.show(ctx),
                Event::Settings => {
                    self.leave_active();
                    self.host.restore(ctx);
                    self.settings = true;
                    self.active = None;
                }
                Event::Quit => self.quit(ctx),
                Event::StopMouse => self.system.stop_mouse(),
                Event::CancelSleep => self.system.cancel_sleep(),
                Event::StopAwake => self.system.stop_awake(),
                Event::StopPresentation => self.system.stop_presentation(),
                Event::StopRecording => self.media.stop_recording(ctx),
                Event::ToggleAwake => self.system.toggle_awake(),
                Event::Lock => self.system.lock_screen(),
                Event::ToggleDesktop => self.system.toggle_desktop(),
            }
        }
        if ctx.input(|input| input.viewport().close_requested()) && !self.quitting {
            if self.integration.has_tray() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.utilities.deactivate();
                self.maintenance.deactivate();
                self.host.hide(ctx);
            } else {
                self.quit(ctx);
            }
        }
        let command = if cfg!(target_os = "macos") {
            egui::Modifiers::MAC_CMD
        } else {
            egui::Modifiers::CTRL
        };
        if crate::shortcuts::consume(ctx, command, egui::Key::K) {
            self.focus_search(ctx);
        }
        if crate::shortcuts::consume(ctx, command, egui::Key::Q) {
            self.quit(ctx);
        }
        match shell_navigation(
            ctx,
            ctx.egui_wants_keyboard_input(),
            self.active.is_some() || self.settings,
            !self.query.is_empty(),
        ) {
            Some(ShellNavigation::FocusSearch) => self.focus_search(ctx),
            Some(ShellNavigation::Back) => {
                self.leave_active();
                self.active = None;
                self.settings = false;
            }
            Some(ShellNavigation::ClearSearch) => self.query.clear(),
            None => {}
        }
        let keys = [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
        ];
        let empty_search = ctx.memory(|memory| memory.focused())
            == Some(egui::Id::unique("search-tools"))
            && self.query.is_empty()
            && self.active.is_none()
            && !self.settings;
        if !ctx.egui_wants_keyboard_input() || empty_search {
            for (index, key) in keys.into_iter().enumerate() {
                if crate::shortcuts::consume(ctx, egui::Modifiers::ALT, key)
                    && let Some(id) = self
                        .catalog
                        .search("favorites", "", &self.prefs.favorites)
                        .get(index)
                        .map(|tool| tool.id.clone())
                {
                    self.leave_active();
                    self.active = Some(id);
                    self.settings = false;
                }
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if self.quitting {
            return;
        }
        self.media.overlays(ui, &mut self.host);
        egui::Panel::left("navigation")
            .exact_size(208.0)
            .show(ui, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    icons::brand(ui, 32.0);
                    ui.label(RichText::new("Oxbox").size(24.0).strong());
                });
                ui.weak("Your everyday toolkit");
                ui.add_space(22.0);
                for (view, label) in [
                    (
                        "favorites",
                        format!(
                            "Dashboard    {}",
                            self.catalog
                                .search("favorites", "", &self.prefs.favorites)
                                .len()
                        ),
                    ),
                    (
                        "all",
                        format!("All tools       {}", self.catalog.tools.len()),
                    ),
                ] {
                    if ui
                        .add_sized(
                            [ui.available_width(), 38.0],
                            egui::Button::new(label)
                                .frame(!self.settings && self.view == view)
                                .selected(!self.settings && self.view == view),
                        )
                        .clicked()
                    {
                        self.select(view);
                    }
                }
                ui.add_space(14.0);
                ui.weak("Explore");
                for category in self.catalog.categories.clone() {
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(&category.label)
                                .frame(!self.settings && self.view == category.id)
                                .selected(!self.settings && self.view == category.id),
                        )
                        .clicked()
                    {
                        self.select(&category.id);
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.weak("Oxbox 0.1.0  ·  Open source");
                    if ui.button("Quit Oxbox").clicked() {
                        self.quit(ui.ctx());
                    }
                    if ui
                        .add(egui::Button::new("Settings").selected(self.settings))
                        .clicked()
                    {
                        self.leave_active();
                        self.settings = true;
                        self.active = None;
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(24))
            .show(ui, |ui| {
                self.system.banners(ui, &mut self.host);
                self.utilities.banners(ui, &mut self.host);
                self.media.recording_banner(ui);
                if let Some(error) = self.error.clone() {
                    ui.horizontal_wrapped(|ui| {
                        crate::accessibility::alert(ui, error);
                        if ui.button("Dismiss").clicked() {
                            self.error = None;
                        }
                    });
                }
                if let Some(error) = &self.integration.error {
                    crate::accessibility::alert(ui, error);
                }
                egui::ScrollArea::vertical()
                    .id_salt("main-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.settings {
                            self.settings_view(ui);
                        } else if let Some(id) = self.active.clone() {
                            if ui.button("← Back to tools").clicked() {
                                self.leave_active();
                                self.active = None;
                                return;
                            }
                            if let Some(tool) = self
                                .catalog
                                .tools
                                .iter()
                                .find(|tool| tool.id == id)
                                .cloned()
                            {
                                ui.heading(&tool.name);
                                ui.label(&tool.description);
                                ui.separator();
                                if !tool.supported() {
                                    ui.label(
                                        "This tool is not available on this operating system.",
                                    );
                                    return;
                                }
                                if !tool.note.is_empty() {
                                    ui.weak(&tool.note);
                                    ui.add_space(10.0);
                                }
                                let current = if id == "resize-images" {
                                    self.media.export_preset(&id)
                                } else {
                                    self.utilities.export_preset(&id)
                                };
                                if let Some(config) =
                                    self.presets.show(ui, &id, &self.host, current)
                                {
                                    let result = if id == "resize-images" {
                                        self.media.import_preset(&config)
                                    } else {
                                        self.utilities.import_preset(&config)
                                    };
                                    match result {
                                        Ok(()) => self.presets.applied(),
                                        Err(error) => self.presets.error = error.to_string(),
                                    }
                                }
                                let handled = self.maintenance.show(ui, &id, &mut self.host)
                                    || self.system.show(ui, &id, &mut self.host)
                                    || self.media.show(ui, &id, &mut self.host)
                                    || self.utilities.show(ui, &id, &mut self.host);
                                if let Some(target) = self.maintenance.take_navigation() {
                                    self.leave_active();
                                    self.active = Some(target);
                                }
                                if !handled {
                                    ui.colored_label(
                                        ui.visuals().error_fg_color,
                                        "This tool's native view failed to register.",
                                    );
                                }
                            }
                        } else {
                            self.browse(ui);
                        }
                    });
            });
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.shutdown();
    }
}

fn capture_id(value: impl Into<u64>) -> u64 {
    value.into()
}
fn native_capture_id(handle: RawWindowHandle) -> u64 {
    match handle {
        RawWindowHandle::Win32(handle) => handle.hwnd.get() as u64,
        RawWindowHandle::Xlib(handle) => capture_id(handle.window),
        RawWindowHandle::Xcb(handle) => capture_id(handle.window.get()),
        // AppKit provides an NSView pointer, not the CGWindowID used by xcap.
        // Keep zero until platform-specific manual acceptance supplies a safe API.
        _ => 0,
    }
}

#[derive(Clone, Copy)]
enum RevealIntent {
    Restore,
    Search,
}
#[derive(Debug, PartialEq, Eq)]
struct RevealPolicy {
    reset_to_launcher: bool,
    focus_search: bool,
}
fn reveal_policy(intent: RevealIntent, active_tool: bool, settings: bool) -> RevealPolicy {
    // Revealing a hidden/unfocused surface is not a navigation request. Only
    // explicit launcher search abandons the active view and its ordinary jobs.
    let reset_to_launcher = matches!(intent, RevealIntent::Search);
    RevealPolicy {
        reset_to_launcher,
        focus_search: reset_to_launcher || (!active_tool && !settings),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ShellNavigation {
    FocusSearch,
    Back,
    ClearSearch,
}

fn shell_navigation(
    ctx: &egui::Context,
    typing: bool,
    details: bool,
    query: bool,
) -> Option<ShellNavigation> {
    // A chooser/context popup receives Escape before Back or search clearing.
    // Leave the event intact so egui can close it and retain the current route.
    if egui::Popup::is_any_open(ctx) {
        return None;
    }
    if (details || query)
        && crate::shortcuts::consume(ctx, egui::Modifiers::NONE, egui::Key::Escape)
    {
        return Some(if details {
            ShellNavigation::Back
        } else {
            ShellNavigation::ClearSearch
        });
    }
    if !typing && crate::shortcuts::consume(ctx, egui::Modifiers::NONE, egui::Key::Slash) {
        // egui-winit emits both a Key and a Text event. Moving focus must not
        // insert the shortcut's slash into the newly focused search field.
        ctx.input_mut(|input| {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Text(text) if text == "/"))
        });
        return Some(ShellNavigation::FocusSearch);
    }
    None
}

#[cfg(test)]
mod navigation_tests {
    use super::*;

    #[test]
    fn shell_popup_escape_dismisses_combo_before_back_or_search_clear() {
        use egui::accesskit::{Action, ActionRequest, Role, TreeId};
        for (details, query) in [(true, false), (false, true)] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let frame = |events| {
                let mut navigation = None;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        // eframe calls shell logic before drawing the current form.
                        navigation = shell_navigation(ui.ctx(), false, details, query);
                        egui::ComboBox::from_label("Generated shell chooser")
                            .selected_text("First")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "First");
                                let _ = ui.selectable_label(false, "Second");
                            });
                    },
                );
                output.textures_delta.clear();
                (
                    navigation,
                    output.platform_output.accesskit_update.unwrap().nodes,
                )
            };
            let (_, nodes) = frame(Vec::new());
            let chooser = nodes
                .into_iter()
                .find_map(|(id, node)| (node.role() == Role::ComboBox).then_some(id))
                .unwrap();
            frame(vec![egui::Event::AccessKitActionRequest(ActionRequest {
                action: Action::Click,
                target_node: chooser,
                target_tree: TreeId::ROOT,
                data: None,
            })]);
            assert!(
                egui::Popup::is_any_open(&context),
                "fixture must open the real chooser popup"
            );
            let (navigation, _) = frame(vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(
                navigation, None,
                "Escape belongs to the open chooser before shell navigation"
            );
            assert!(
                !egui::Popup::is_any_open(&context),
                "the chooser must receive Escape and close"
            );
        }
    }

    #[test]
    fn restoring_a_tool_or_settings_does_not_navigate_or_cancel_its_work() {
        for (active_tool, settings) in [(true, false), (false, true), (true, true)] {
            assert_eq!(
                reveal_policy(RevealIntent::Restore, active_tool, settings),
                RevealPolicy {
                    reset_to_launcher: false,
                    focus_search: false,
                }
            );
        }
        assert_eq!(
            reveal_policy(RevealIntent::Restore, false, false),
            RevealPolicy {
                reset_to_launcher: false,
                focus_search: true,
            }
        );
    }

    #[test]
    fn explicit_search_navigation_still_leaves_the_previous_surface() {
        for (active_tool, settings) in [(false, false), (true, false), (false, true), (true, true)]
        {
            assert_eq!(
                reveal_policy(RevealIntent::Search, active_tool, settings),
                RevealPolicy {
                    reset_to_launcher: true,
                    focus_search: true,
                }
            );
        }
    }

    #[test]
    fn capture_identity_uses_native_window_ids_and_never_an_appkit_view_pointer() {
        use raw_window_handle::{
            AppKitWindowHandle, Win32WindowHandle, XcbWindowHandle, XlibWindowHandle,
        };
        assert_eq!(
            native_capture_id(RawWindowHandle::Xlib(XlibWindowHandle::new(47))),
            47
        );
        assert_eq!(
            native_capture_id(RawWindowHandle::Xcb(XcbWindowHandle::new(
                std::num::NonZeroU32::new(48).unwrap()
            ))),
            48
        );
        assert_eq!(
            native_capture_id(RawWindowHandle::Win32(Win32WindowHandle::new(
                std::num::NonZeroIsize::new(49).unwrap()
            ))),
            49
        );
        assert_eq!(
            native_capture_id(RawWindowHandle::AppKit(AppKitWindowHandle::new(
                std::ptr::NonNull::dangling()
            ))),
            0
        );
    }

    fn action(key: egui::Key, typing: bool, details: bool, query: bool) -> Option<ShellNavigation> {
        let context = egui::Context::default();
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut result = None;
        let mut output = context.run_ui(raw, |ui| {
            result = shell_navigation(ui.ctx(), typing, details, query);
        });
        output.textures_delta.clear();
        result
    }
    #[test]
    fn slash_focuses_search_only_outside_text_input() {
        assert_eq!(
            action(egui::Key::Slash, false, false, false),
            Some(ShellNavigation::FocusSearch)
        );
        assert_eq!(action(egui::Key::Slash, true, false, false), None);
    }
    #[test]
    fn slash_shortcut_consumes_paired_text_only_when_it_moves_focus() {
        for typing in [false, true] {
            let context = egui::Context::default();
            let mut raw = egui::RawInput::default();
            raw.events.push(egui::Event::Key {
                key: egui::Key::Slash,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            raw.events.push(egui::Event::Text("/".into()));
            let mut output = context.run_ui(raw, |ui| {
                let _ = shell_navigation(ui.ctx(), typing, false, false);
                let text_remains = ui.input(|input| {
                    input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::Text(text) if text == "/"))
                });
                assert_eq!(text_remains, typing);
            });
            output.textures_delta.clear();
        }
    }
    #[test]
    fn escape_returns_from_tools_or_settings_before_clearing_search() {
        assert_eq!(
            action(egui::Key::Escape, true, true, true),
            Some(ShellNavigation::Back)
        );
        assert_eq!(
            action(egui::Key::Escape, true, false, true),
            Some(ShellNavigation::ClearSearch)
        );
        assert_eq!(action(egui::Key::Escape, false, false, false), None);
    }
}
