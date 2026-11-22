use super::{field, issue};
use crate::host::{self, Host};
use eframe::egui::{self, Ui};
use oxbox_core::{
    CoreError, CoreResult,
    tools::{
        alarm::{Alarm, AlarmBook, AlarmInput, Ambiguity, Recurrence},
        break_time::{BreakScheduler, BreakSettings},
        date_countdown::{Countdown, CountdownBook, CountdownInput},
        timer_stopwatch::{Clock, format_time},
        world_time,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub(super) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs().min(i64::MAX as u64) as i64)
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Stored {
    alarms: Vec<Alarm>,
    countdowns: Vec<Countdown>,
    world_zones: Vec<String>,
    break_settings: BreakSettings,
}
struct Runtime {
    alarms: AlarmBook,
    countdowns: CountdownBook,
    zones: Vec<String>,
    settings: BreakSettings,
    breaks: BreakScheduler,
    timer: Clock,
    stopwatch: Clock,
    origin: Instant,
    error: String,
    notice: String,
    ready: bool,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            alarms: AlarmBook::default(),
            countdowns: CountdownBook::default(),
            zones: Vec::new(),
            settings: BreakSettings::default(),
            breaks: BreakScheduler::default(),
            timer: Clock::timer(25 * 60 * 1000).unwrap_or_default(),
            stopwatch: Clock::default(),
            origin: Instant::now(),
            error: String::new(),
            notice: String::new(),
            ready: false,
        }
    }
}
impl Runtime {
    fn stored(&self) -> Stored {
        Stored {
            alarms: self.alarms.items.clone(),
            countdowns: self.countdowns.items.clone(),
            world_zones: self.zones.clone(),
            break_settings: self.settings.clone(),
        }
    }
    fn ms(&self) -> u64 {
        self.origin.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
}
struct Draft {
    label: String,
    date: String,
    zone: String,
    recurrence: Recurrence,
    later: bool,
    notify: bool,
}
impl Default for Draft {
    fn default() -> Self {
        Self {
            label: String::new(),
            date: (chrono::Utc::now() + chrono::Duration::hours(1))
                .format("%Y-%m-%dT%H:%M")
                .to_string(),
            zone: "UTC".into(),
            recurrence: Recurrence::Once,
            later: false,
            notify: true,
        }
    }
}
pub(super) struct TimeTools {
    state: Arc<Mutex<Runtime>>,
    worker: Option<JoinHandle<()>>,
    stopping: Arc<AtomicBool>,
    loaded: bool,
    stopwatch: bool,
    minutes: f64,
    query: String,
    zone: String,
    alarm: Draft,
    countdown: Draft,
    break_minutes: u64,
    eye_rest: bool,
    pause_fullscreen: bool,
}
impl Default for TimeTools {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(Runtime::default())),
            worker: None,
            stopping: Arc::new(AtomicBool::new(false)),
            loaded: false,
            stopwatch: false,
            minutes: 25.0,
            query: String::new(),
            zone: "UTC".into(),
            alarm: Draft::default(),
            countdown: Draft::default(),
            break_minutes: 20,
            eye_rest: false,
            pause_fullscreen: true,
        }
    }
}
fn validate(stored: &Stored) -> CoreResult<()> {
    AlarmBook::restore(stored.alarms.clone())?;
    CountdownBook::restore(stored.countdowns.clone())?;
    if stored.world_zones.len() > 200 {
        return Err(CoreError::InvalidInput(
            "At most 200 world clocks are supported.".into(),
        ));
    }
    for zone in &stored.world_zones {
        world_time::clock_at(zone, 0)?;
    }
    let mut scheduler = BreakScheduler::default();
    scheduler.start(
        0,
        stored.break_settings.minutes,
        stored.break_settings.eye_rest,
    )?;
    Ok(())
}
impl TimeTools {
    fn initialize(&mut self, host: &mut Host) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let loaded = (|| {
            if let Some(stored) = host.load::<Stored>("native-utilities.json")? {
                validate(&stored)?;
                return Ok(stored);
            }
            let legacy = host
                .load::<serde_json::Value>("settings.json")?
                .unwrap_or_default();
            let alarms = legacy
                .pointer("/roadmapTime/alarms")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::InvalidInput(format!("Could not import alarms: {error}"))
                })?
                .unwrap_or_default();
            let countdowns = legacy
                .pointer("/roadmapTime/countdowns")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::InvalidInput(format!("Could not import date countdowns: {error}"))
                })?
                .unwrap_or_default();
            let world_zones = legacy
                .pointer("/preferences/worldZones")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::InvalidInput(format!("Could not import world clocks: {error}"))
                })?
                .unwrap_or_default();
            let break_settings = legacy
                .get("breakTime")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::InvalidInput(format!("Could not import break settings: {error}"))
                })?
                .unwrap_or_default();
            let stored = Stored {
                alarms,
                countdowns,
                world_zones,
                break_settings,
            };
            validate(&stored)?;
            host.save("native-utilities.json", &stored)?;
            Ok::<_, CoreError>(stored)
        })();
        if let Ok(mut state) = self.state.lock() {
            match loaded {
                Ok(stored) => {
                    state.alarms = AlarmBook::restore(stored.alarms).unwrap_or_default();
                    state.countdowns =
                        CountdownBook::restore(stored.countdowns).unwrap_or_default();
                    state.zones = stored.world_zones;
                    state.settings = stored.break_settings;
                    self.break_minutes = state.settings.minutes;
                    self.eye_rest = state.settings.eye_rest;
                    self.pause_fullscreen = state.settings.pause_fullscreen;
                    if state.settings.enabled {
                        let settings = state.settings.clone();
                        let _ = state.breaks.start(0, settings.minutes, settings.eye_rest);
                    }
                    state.ready = true;
                }
                Err(error) => {
                    state.error = format!(
                        "Reminder settings could not be loaded. Preserve and repair the original files before editing: {error}"
                    )
                }
            }
        }
        let state = self.state.clone();
        let stopping = self.stopping.clone();
        let notifier = host.notifier.clone();
        let directory = host.data_dir().to_path_buf();
        match std::thread::Builder::new()
            .name("oxbox-native-reminders".into())
            .spawn(move || {
                let mut last = i64::MIN;
                while !stopping.load(Ordering::Acquire) {
                    if let Ok(mut state) = state.lock() {
                        if stopping.load(Ordering::Acquire) {
                            break;
                        }
                        let ms = state.ms();
                        if state.timer.finish(ms) {
                            state.notice = "Timer complete.".into();
                            if !stopping.load(Ordering::Acquire) {
                                notifier
                                    .notify("Timer complete", "Your Oxbox countdown has finished.");
                            }
                        }
                        if state.ready {
                            let now = unix_now();
                            if now != last {
                                last = now;
                                let old_alarms = state.alarms.clone();
                                let old_dates = state.countdowns.clone();
                                let alarms = state.alarms.tick(now);
                                let dates = state.countdowns.tick(now);
                                if !alarms.is_empty() || !dates.is_empty() {
                                    match host::save_at(
                                        &directory,
                                        "native-utilities.json",
                                        &state.stored(),
                                    ) {
                                        Ok(()) => {
                                            state.error.clear();
                                            if !stopping.load(Ordering::Acquire) {
                                                for alarm in alarms {
                                                    notifier
                                                        .notify("Oxbox alarm", &alarm.input.label);
                                                }
                                                for date in dates {
                                                    notifier
                                                        .notify("Date reached", &date.input.label);
                                                }
                                            }
                                        }
                                        Err(error) => {
                                            state.alarms = old_alarms;
                                            state.countdowns = old_dates;
                                            state.error = error.to_string();
                                        }
                                    }
                                }
                                let inactive = state.settings.enabled
                                    && oxbox_core::platform::activity_paused(
                                        state.settings.pause_fullscreen,
                                    );
                                if state.breaks.tick(ms / 1000, inactive)
                                    && !stopping.load(Ordering::Acquire)
                                {
                                    notifier.notify(
                                        "Time for a break",
                                        if state.settings.eye_rest {
                                            "Look 20 feet away for 20 seconds."
                                        } else {
                                            "Take a short break, stretch and rest your eyes."
                                        },
                                    );
                                }
                            }
                        }
                    }
                    std::thread::park_timeout(Duration::from_millis(250));
                }
            }) {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => {
                if let Ok(mut state) = self.state.lock() {
                    state.error = format!("Background reminders could not start: {error}");
                    state.ready = false;
                }
            }
        }
    }
    pub fn tick(&mut self, host: &mut Host) {
        self.initialize(host);
    }
    pub fn shutdown(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
    pub fn shutdown_request(&self) -> Arc<AtomicBool> {
        self.stopping.clone()
    }
    pub fn banners(&mut self, ui: &mut Ui, host: &mut Host) {
        self.initialize(host);
        if let Ok(mut state) = self.state.lock() {
            if !state.error.is_empty() {
                issue(ui, format!("Reminders: {}", state.error));
            }
            let ms = state.ms();
            if state.timer.running() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Timer: {} remaining",
                        format_time(state.timer.remaining(ms))
                    ));
                    if ui.button("Pause timer").clicked() {
                        state.timer.pause(ms);
                    }
                    if ui.button("Reset timer").clicked() {
                        state.timer.reset();
                        state.notice.clear();
                    }
                });
            } else if state.timer.completed() {
                ui.horizontal(|ui| {
                    ui.label("Timer complete.");
                    if ui.button("Dismiss timer completion").clicked() {
                        state.timer.reset();
                        state.notice.clear();
                    }
                });
            }
            let status = state.breaks.status(ms / 1000);
            if status.awaiting_break {
                ui.horizontal(|ui| {
                    ui.label(if status.eye_rest {
                        "Break time: rest your eyes for 20 seconds."
                    } else {
                        "Break time: stand up, stretch and rest your eyes."
                    });
                    if ui.button("Snooze 5 minutes").clicked() {
                        state.breaks.snooze(ms / 1000);
                    }
                    if ui.button("Continue / skip break").clicked() {
                        state.breaks.skip(ms / 1000);
                    }
                });
            }
        }
        let ringing = self
            .state
            .lock()
            .map(|state| {
                state
                    .alarms
                    .items
                    .iter()
                    .filter(|alarm| alarm.ringing)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut action = None;
        for alarm in ringing {
            ui.push_id(("alarm-banner", alarm.id), |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Alarm: {}", alarm.input.label));
                    if crate::accessibility::button(
                        ui,
                        "Snooze 5 minutes",
                        &format!("Snooze alarm {} for 5 minutes", alarm.input.label),
                    )
                    .clicked()
                    {
                        action = Some((alarm.id, true));
                    }
                    if crate::accessibility::button(
                        ui,
                        "Dismiss alarm",
                        &format!("Dismiss alarm {}", alarm.input.label),
                    )
                    .clicked()
                    {
                        action = Some((alarm.id, false));
                    }
                });
            });
        }
        if let Some((id, snooze)) = action {
            self.books(host, move |alarms, _| {
                if snooze {
                    alarms.snooze(id, unix_now(), 5)
                } else {
                    alarms.dismiss(id)
                }
            });
        }
    }
    pub fn show(&mut self, ui: &mut Ui, id: &str, host: &mut Host) {
        self.initialize(host);
        match id {
            "timer-stopwatch" => self.clocks(ui),
            "world-time" => self.world(ui, host),
            "alarm" => self.alarms(ui, host),
            "date-countdown" => self.dates(ui, host),
            "break-time" => self.breaks(ui, host),
            _ => {}
        }
        if let Ok(state) = self.state.lock() {
            if !state.error.is_empty() {
                issue(ui, &state.error);
            }
            if !state.notice.is_empty() {
                ui.label(&state.notice);
            }
        }
    }
    fn clocks(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.stopwatch, false, "Timer");
            ui.selectable_value(&mut self.stopwatch, true, "Stopwatch");
        });
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let now = state.ms();
        let mut clear_notice = false;
        let clock = if self.stopwatch {
            &mut state.stopwatch
        } else {
            &mut state.timer
        };
        if !self.stopwatch {
            ui.horizontal(|ui| {
                for minutes in [1, 5, 10, 25] {
                    if ui.button(format!("{minutes} min")).clicked() {
                        clear_notice = true;
                        self.minutes = minutes as f64;
                        if let Ok(timer) = Clock::timer(minutes * 60 * 1000) {
                            *clock = timer;
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                let label = ui.label("Duration (minutes)");
                ui.add_enabled(
                    !clock.running() && (clock.elapsed(now) == 0 || clock.completed()),
                    egui::DragValue::new(&mut self.minutes)
                        .range(0.01..=10080.0)
                        .speed(0.1),
                )
                .labelled_by(label.id);
            });
        }
        ui.heading(format_time(if self.stopwatch {
            clock.elapsed(now)
        } else {
            clock.remaining(now)
        }));
        let keyboard = !ui.ctx().egui_wants_keyboard_input();
        let valid = self.stopwatch
            || (self.minutes.is_finite() && self.minutes > 0.0 && self.minutes <= 10080.0);
        if !valid {
            issue(
                ui,
                "Choose a duration greater than zero and no longer than 10080 minutes.",
            );
        }
        let toggle_key = keyboard
            && valid
            && crate::shortcuts::consume(ui.ctx(), egui::Modifiers::NONE, egui::Key::Space);
        let reset_key =
            keyboard && crate::shortcuts::consume(ui.ctx(), egui::Modifiers::NONE, egui::Key::R);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    valid,
                    egui::Button::new(if clock.running() { "Pause" } else { "Start" }),
                )
                .clicked()
                || toggle_key
            {
                clear_notice = true;
                if clock.running() {
                    clock.pause(now);
                } else {
                    if !self.stopwatch
                        && (clock.elapsed(now) == 0 || clock.completed())
                        && valid
                        && let Ok(timer) = Clock::timer((self.minutes * 60000.0).round() as u64)
                    {
                        *clock = timer;
                    }
                    clock.start(now);
                }
            }
            if ui.button("Reset").clicked() || reset_key {
                clear_notice = true;
                clock.reset();
            }
            if self.stopwatch
                && ui
                    .add_enabled(
                        clock.running() && clock.laps.len() < 1000,
                        egui::Button::new("Lap"),
                    )
                    .clicked()
            {
                let _ = clock.lap(now);
            }
        });
        ui.label("Space starts/pauses; R resets when not editing a control. Clocks continue while browsing or hiding the launcher.");
        for (index, lap) in clock.laps.iter().enumerate() {
            ui.monospace(format!(
                "Lap {}: {}.{:03}",
                index + 1,
                format_time(*lap),
                lap % 1000
            ));
        }
        if clear_notice {
            state.notice.clear();
        }
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    fn world(&mut self, ui: &mut Ui, host: &mut Host) {
        field(ui, "Search IANA time zones", &mut self.query);
        let query = self.query.to_lowercase();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let matches = world_time::zones()
            .into_iter()
            .filter(|zone| {
                !state.zones.iter().any(|current| current == zone)
                    && zone.to_lowercase().replace('_', " ").contains(&query)
            })
            .take(100)
            .collect::<Vec<_>>();
        if !matches.contains(&self.zone.as_str()) {
            self.zone = matches.first().copied().unwrap_or("").into();
        }
        egui::ComboBox::from_label("Time zone to add")
            .selected_text(&self.zone)
            .show_ui(ui, |ui| {
                for zone in &matches {
                    ui.selectable_value(&mut self.zone, zone.to_string(), zone.replace('_', " "));
                }
            });
        let previous = state.zones.clone();
        if ui
            .add_enabled(
                state.ready && !self.zone.is_empty() && state.zones.len() < 200,
                egui::Button::new("Add clock"),
            )
            .clicked()
        {
            state.zones.push(self.zone.clone());
        }
        let mut action = None;
        for (index, zone) in state.zones.iter().enumerate() {
            ui.group(|ui| {
                if let Ok(clock) = world_time::now(zone) {
                    ui.heading(zone.replace('_', " "));
                    ui.monospace(clock.time);
                    let minutes = clock.offset_seconds / 60;
                    let local = chrono::Local::now().offset().local_minus_utc();
                    ui.label(format!(
                        "{} · {} · UTC{}{:02}:{:02} · {} · {:+} minutes from local",
                        clock.date,
                        clock.abbreviation,
                        if minutes < 0 { "−" } else { "+" },
                        minutes.unsigned_abs() / 60,
                        minutes.unsigned_abs() % 60,
                        if clock.daytime { "Day" } else { "Night" },
                        (clock.offset_seconds - local) / 60
                    ));
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled_ui(index > 0, |ui| {
                            crate::accessibility::button(
                                ui,
                                "Up",
                                &format!("Move world clock {zone} up"),
                            )
                        })
                        .inner
                        .clicked()
                    {
                        action = Some((index, -1));
                    }
                    if ui
                        .add_enabled_ui(index + 1 < state.zones.len(), |ui| {
                            crate::accessibility::button(
                                ui,
                                "Down",
                                &format!("Move world clock {zone} down"),
                            )
                        })
                        .inner
                        .clicked()
                    {
                        action = Some((index, 1));
                    }
                    if crate::accessibility::button(
                        ui,
                        "Remove",
                        &format!("Remove world clock {zone}"),
                    )
                    .clicked()
                    {
                        action = Some((index, 0));
                    }
                });
            });
        }
        if let Some((index, direction)) = action {
            if direction == 0 {
                state.zones.remove(index);
            } else {
                state
                    .zones
                    .swap(index, (index as isize + direction) as usize);
            }
        }
        if previous != state.zones
            && let Err(error) = host.save("native-utilities.json", &state.stored())
        {
            state.zones = previous;
            state.error = error.to_string();
        }
        ui.label("Showing up to 100 matching zones. Offsets use bundled IANA rules, including daylight saving.");
        ui.ctx().request_repaint_after(Duration::from_secs(1));
    }
    fn books(
        &self,
        host: &Host,
        change: impl FnOnce(&mut AlarmBook, &mut CountdownBook) -> CoreResult<()>,
    ) {
        if let Ok(mut state) = self.state.lock() {
            if !state.ready {
                return;
            }
            let old_alarms = state.alarms.clone();
            let old_dates = state.countdowns.clone();
            let Runtime {
                alarms, countdowns, ..
            } = &mut *state;
            let result = change(alarms, countdowns)
                .and_then(|()| host.save("native-utilities.json", &state.stored()));
            if let Err(error) = result {
                state.alarms = old_alarms;
                state.countdowns = old_dates;
                state.error = error.to_string();
            } else {
                state.error.clear();
            }
        }
    }
    fn draft(ui: &mut Ui, draft: &mut Draft) {
        field(ui, "Name", &mut draft.label);
        field(
            ui,
            "Local date and time (YYYY-MM-DDTHH:MM)",
            &mut draft.date,
        );
        field(ui, "IANA time zone", &mut draft.zone);
        egui::ComboBox::from_label("Choose IANA zone")
            .selected_text(&draft.zone)
            .show_ui(ui, |ui| {
                for zone in world_time::zones() {
                    ui.selectable_value(&mut draft.zone, zone.to_string(), zone.replace('_', " "));
                }
            });
        ui.checkbox(
            &mut draft.later,
            "Use the later occurrence when daylight saving repeats this local time",
        );
        ui.label("Times skipped by a daylight-saving jump are rejected. A repeated time uses the earlier occurrence unless selected otherwise.");
    }
    fn alarms(&mut self, ui: &mut Ui, host: &mut Host) {
        Self::draft(ui, &mut self.alarm);
        egui::ComboBox::from_label("Repeat")
            .selected_text(format!("{:?}", self.alarm.recurrence))
            .show_ui(ui, |ui| {
                for (value, label) in [
                    (Recurrence::Once, "Once"),
                    (Recurrence::Daily, "Daily"),
                    (Recurrence::Weekdays, "Weekdays"),
                ] {
                    ui.selectable_value(&mut self.alarm.recurrence, value, label);
                }
            });
        if ui.button("Add alarm").clicked() {
            let input = AlarmInput {
                label: self.alarm.label.clone(),
                local_date_time: self.alarm.date.clone(),
                zone: self.alarm.zone.clone(),
                recurrence: self.alarm.recurrence,
                ambiguity: if self.alarm.later {
                    Ambiguity::Later
                } else {
                    Ambiguity::Earlier
                },
            };
            self.books(host, move |alarms, _| {
                alarms.add(input, unix_now()).map(|_| ())
            });
        }
        let items = self
            .state
            .lock()
            .map(|state| state.alarms.items.clone())
            .unwrap_or_default();
        let mut action = None;
        for alarm in items {
            ui.group(|ui| {
                ui.heading(&alarm.input.label);
                let next = world_time::clock_at(&alarm.input.zone, alarm.next_at)
                    .map(|clock| format!("{} {} {}", clock.date, clock.time, clock.abbreviation))
                    .unwrap_or_else(|_| alarm.next_at.to_string());
                ui.label(format!(
                    "{} · {} · {:?} · next {}{}",
                    alarm.input.local_date_time,
                    alarm.input.zone,
                    alarm.input.recurrence,
                    next,
                    if alarm.ringing { " · RINGING" } else { "" }
                ));
                ui.horizontal(|ui| {
                    let verb = if alarm.enabled { "Disable" } else { "Enable" };
                    if crate::accessibility::button(
                        ui,
                        verb,
                        &format!("{verb} alarm {}", alarm.input.label),
                    )
                    .clicked()
                    {
                        action = Some((alarm.id, 0, !alarm.enabled));
                    }
                    if alarm.ringing
                        && crate::accessibility::button(
                            ui,
                            "Snooze 5 minutes",
                            &format!("Snooze alarm {} for 5 minutes", alarm.input.label),
                        )
                        .clicked()
                    {
                        action = Some((alarm.id, 1, true));
                    }
                    if alarm.ringing
                        && crate::accessibility::button(
                            ui,
                            "Dismiss",
                            &format!("Dismiss alarm {}", alarm.input.label),
                        )
                        .clicked()
                    {
                        action = Some((alarm.id, 2, true));
                    }
                    if crate::accessibility::button(
                        ui,
                        "Remove",
                        &format!("Remove alarm {}", alarm.input.label),
                    )
                    .clicked()
                    {
                        action = Some((alarm.id, 3, false));
                    }
                });
            });
        }
        if let Some((id, kind, enabled)) = action {
            self.books(host, move |alarms, _| match kind {
                0 => alarms.set_enabled(id, enabled, unix_now()),
                1 => alarms.snooze(id, unix_now(), 5),
                2 => alarms.dismiss(id),
                _ => alarms.remove(id),
            });
        }
        ui.label("Alarms and ringing state persist. Due notifications run while the launcher is hidden; recurring alarms follow the chosen IANA zone.");
    }
    fn dates(&mut self, ui: &mut Ui, host: &mut Host) {
        Self::draft(ui, &mut self.countdown);
        ui.checkbox(
            &mut self.countdown.notify,
            "Notify once when this date is reached",
        );
        if ui.button("Add date countdown").clicked() {
            let input = CountdownInput {
                label: self.countdown.label.clone(),
                local_date_time: self.countdown.date.clone(),
                zone: self.countdown.zone.clone(),
                notify: self.countdown.notify,
                ambiguity: if self.countdown.later {
                    Ambiguity::Later
                } else {
                    Ambiguity::Earlier
                },
            };
            self.books(host, move |_, dates| {
                dates.add(input, unix_now()).map(|_| ())
            });
        }
        let snapshots = self
            .state
            .lock()
            .map(|state| state.countdowns.snapshot(unix_now()))
            .unwrap_or_default();
        let mut remove = None;
        for snapshot in snapshots {
            ui.group(|ui| {
                ui.heading(&snapshot.item.input.label);
                let seconds = snapshot.remaining_seconds;
                ui.monospace(format!(
                    "{} days {} {}",
                    seconds.unsigned_abs() / 86400,
                    format_time((seconds.unsigned_abs() % 86400).saturating_mul(1000)),
                    if seconds < 0 { "ago" } else { "remaining" }
                ));
                ui.label(format!(
                    "{} · {}{}",
                    snapshot.item.input.local_date_time,
                    snapshot.item.input.zone,
                    if snapshot.item.notified {
                        " · notification already delivered/not requested for past date"
                    } else {
                        ""
                    }
                ));
                if crate::accessibility::button(
                    ui,
                    "Remove",
                    &format!("Remove date countdown {}", snapshot.item.input.label),
                )
                .clicked()
                {
                    remove = Some(snapshot.item.id);
                }
            });
        }
        if let Some(id) = remove {
            self.books(host, move |_, dates| dates.remove(id));
        }
        ui.ctx().request_repaint_after(Duration::from_secs(1));
    }
    fn breaks(&mut self, ui: &mut Ui, host: &mut Host) {
        ui.horizontal(|ui| {
            let label = ui.label("Active-work interval (minutes)");
            ui.add(egui::DragValue::new(&mut self.break_minutes).range(1..=1440))
                .labelled_by(label.id);
        });
        ui.checkbox(&mut self.eye_rest, "20-20-20 eye-rest reminder");
        ui.checkbox(
            &mut self.pause_fullscreen,
            "Pause during fullscreen (best effort)",
        );
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let now = state.ms() / 1000;
        let status = state.breaks.status(now);
        ui.label(if !status.enabled {
            "Break reminders are off."
        } else if status.awaiting_break {
            "Time for a break."
        } else if status.paused {
            "Paused while idle/fullscreen."
        } else {
            "Counting active work time."
        });
        ui.label(format!(
            "Next reminder in {} seconds",
            status.remaining_seconds
        ));
        let start = ui
            .add_enabled(
                state.ready,
                egui::Button::new(if status.enabled {
                    "Save and restart interval"
                } else {
                    "Start reminders"
                }),
            )
            .clicked();
        let stop = status.enabled && ui.button("Stop reminders").clicked();
        if start || stop {
            let settings = BreakSettings {
                minutes: self.break_minutes,
                eye_rest: self.eye_rest,
                pause_fullscreen: self.pause_fullscreen,
                enabled: !stop,
            };
            let mut candidate = BreakScheduler::default();
            let result = candidate
                .start(now, settings.minutes, settings.eye_rest)
                .and_then(|()| {
                    let old = state.settings.clone();
                    state.settings = settings.clone();
                    let saved = host.save("native-utilities.json", &state.stored());
                    if saved.is_err() {
                        state.settings = old;
                    }
                    saved
                });
            match result {
                Ok(()) => {
                    if !settings.enabled {
                        candidate.stop();
                    }
                    state.breaks = candidate;
                    state.error.clear();
                }
                Err(error) => state.error = error.to_string(),
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(status.enabled, egui::Button::new("Snooze 5 minutes"))
                .clicked()
            {
                state.breaks.snooze(now);
            }
            if ui
                .add_enabled(status.enabled, egui::Button::new("Skip this break"))
                .clicked()
            {
                state.breaks.skip(now);
            }
        });
        ui.label("Saved reminder settings resume after restart. Windows idle/fullscreen detection is best effort; other platforms may lack the same signals.");
        ui.ctx().request_repaint_after(Duration::from_secs(1));
    }
}
impl Drop for TimeTools {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;

    fn names(nodes: &[egui::accesskit::Node]) -> Vec<&str> {
        nodes
            .iter()
            .filter(|node| node.role() == egui::accesskit::Role::Button)
            .filter_map(|node| node.label())
            .collect()
    }

    #[test]
    fn two_time_rows_have_distinct_named_actions_without_starting_workers() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut tools = TimeTools::default();
        tools.loaded = true;
        {
            let mut state = tools.state.lock().unwrap();
            state.ready = true;
            state.zones = vec!["UTC".into(), "Asia/Tokyo".into()];
            for (id, label) in [(1, "Morning"), (2, "Evening")] {
                state.alarms.items.push(Alarm {
                    id,
                    input: AlarmInput {
                        label: label.into(),
                        local_date_time: "2100-01-01T12:00".into(),
                        zone: "UTC".into(),
                        recurrence: Recurrence::Once,
                        ambiguity: Ambiguity::Earlier,
                    },
                    next_at: 4_102_444_800,
                    enabled: id == 1,
                    ringing: true,
                    last_fired: None,
                });
                state.countdowns.items.push(Countdown {
                    id,
                    input: CountdownInput {
                        label: label.into(),
                        local_date_time: "2100-01-01T12:00".into(),
                        zone: "UTC".into(),
                        notify: false,
                        ambiguity: Ambiguity::Earlier,
                    },
                    target: 4_102_444_800,
                    notified: true,
                });
            }
        }
        let alarms = crate::accessibility::test_tree(|ui| tools.alarms(ui, &mut host));
        for label in ["Morning", "Evening"] {
            for expected in [
                format!("Remove alarm {label}"),
                format!("Snooze alarm {label} for 5 minutes"),
                format!("Dismiss alarm {label}"),
            ] {
                assert!(
                    names(&alarms).contains(&expected.as_str()),
                    "missing alarm action {expected}"
                );
            }
        }
        assert!(names(&alarms).contains(&"Disable alarm Morning"));
        assert!(names(&alarms).contains(&"Enable alarm Evening"));
        let banners = crate::accessibility::test_tree(|ui| tools.banners(ui, &mut host));
        for label in ["Morning", "Evening"] {
            assert!(
                names(&banners).contains(&format!("Snooze alarm {label} for 5 minutes").as_str())
            );
            assert!(names(&banners).contains(&format!("Dismiss alarm {label}").as_str()));
        }
        let dates = crate::accessibility::test_tree(|ui| tools.dates(ui, &mut host));
        for label in ["Morning", "Evening"] {
            assert!(names(&dates).contains(&format!("Remove date countdown {label}").as_str()));
        }
        let world = crate::accessibility::test_tree(|ui| tools.world(ui, &mut host));
        for zone in ["UTC", "Asia/Tokyo"] {
            assert!(names(&world).contains(&format!("Remove world clock {zone}").as_str()));
            assert!(names(&world).contains(&format!("Move world clock {zone} up").as_str()));
            assert!(names(&world).contains(&format!("Move world clock {zone} down").as_str()));
        }
        assert!(tools.worker.is_none());
        assert_eq!(tools.state.lock().unwrap().alarms.items.len(), 2);
        tools.shutdown();
        host.shutdown();
    }
}
