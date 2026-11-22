use super::runtime;
use crate::jobs::Task;
use eframe::egui;
use oxbox_core::tools::{
    cpu_temperature,
    network::{InterfaceUsage, NetworkMonitor},
    system_monitor::{Monitor, Snapshot},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub struct MonitorViews {
    monitor: Arc<Mutex<Monitor>>,
    network: Arc<Mutex<Option<NetworkMonitor>>>,
    snapshot: Task<Snapshot>,
    traffic: Task<Vec<InterfaceUsage>>,
    sensors: Task<Vec<cpu_temperature::Sensor>>,
    last: Instant,
    last_network: Instant,
    sampling_origin: Instant,
    last_sensor_ms: Option<u64>,
    history: VecDeque<f32>,
    virtuals: bool,
}
impl Default for MonitorViews {
    fn default() -> Self {
        Self {
            monitor: Arc::new(Mutex::new(Monitor::default())),
            network: Arc::new(Mutex::new(None)),
            snapshot: Default::default(),
            traffic: Default::default(),
            sensors: Default::default(),
            last: Instant::now() - Duration::from_secs(2),
            last_network: Instant::now() - Duration::from_secs(2),
            sampling_origin: Instant::now(),
            last_sensor_ms: None,
            history: VecDeque::new(),
            virtuals: false,
        }
    }
}
impl MonitorViews {
    pub fn poll(&mut self) {
        let busy = self.snapshot.running();
        self.snapshot.poll();
        if busy
            && !self.snapshot.running()
            && let Some(s) = &self.snapshot.result
        {
            self.history.push_back(s.cpu_percent);
            if self.history.len() > 60 {
                self.history.pop_front();
            }
        }
        self.traffic.poll();
        self.sensors.poll();
    }
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str) -> bool {
        match id {
            "system-monitor" => {
                ui.label("Local CPU, memory, disk and process statistics. Sampling runs only while this view is open.");
                if self.last.elapsed() >= Duration::from_secs(1) && !self.snapshot.running() {
                    self.last = Instant::now();
                    let monitor = self.monitor.clone();
                    self.snapshot.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        Ok(runtime::lock(&monitor)?.snapshot())
                    });
                }
                if let Some(s) = &self.snapshot.result {
                    ui.heading(format!("CPU {:.1}%", s.cpu_percent));
                    let (size, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 80.0),
                        egui::Sense::hover(),
                    );
                    let _ = response;
                    if self.history.len() > 1 {
                        let points = self
                            .history
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                egui::pos2(
                                    size.left()
                                        + i as f32 * size.width() / (self.history.len() - 1) as f32,
                                    size.bottom() - v / 100.0 * size.height(),
                                )
                            })
                            .collect();
                        ui.painter().add(egui::Shape::line(
                            points,
                            egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
                        ));
                    }
                    ui.label("Last 60 samples");
                    ui.horizontal_wrapped(|ui| {
                        for (i, cpu) in s.cores.iter().enumerate() {
                            ui.label(format!("Core {}: {:.0}%", i + 1, cpu));
                        }
                    });
                    ui.separator();
                    ui.label(format!(
                        "Memory {} / {} · swap {} / {}",
                        bytes(s.memory_used),
                        bytes(s.memory_total),
                        bytes(s.swap_used),
                        bytes(s.swap_total)
                    ));
                    let section = egui::CollapsingHeader::new("Disks")
                        .default_open(true)
                        .show(ui, |ui| {
                            for disk in &s.disks {
                                ui.label(format!(
                                    "{} ({}) · {} used / {} total",
                                    disk.name,
                                    disk.mount,
                                    bytes(disk.total.saturating_sub(disk.available)),
                                    bytes(disk.total)
                                ));
                            }
                        });
                    crate::accessibility::disclosure_expansion(&section.header_response);
                    for (title, rows) in [
                        ("Top processes by CPU", &s.processes_cpu),
                        ("Top processes by memory", &s.processes_memory),
                    ] {
                        let section =
                            egui::CollapsingHeader::new(title)
                                .default_open(true)
                                .show(ui, |ui| {
                                    egui::Grid::new(title).striped(true).show(ui, |ui| {
                                        ui.strong("Process / PID");
                                        ui.strong("CPU");
                                        ui.strong("Memory");
                                        ui.end_row();
                                        for p in rows {
                                            ui.label(format!("{} / {}", p.name, p.pid));
                                            ui.label(format!("{:.1}%", p.cpu_percent));
                                            ui.label(bytes(p.memory));
                                            ui.end_row();
                                        }
                                    });
                                });
                        crate::accessibility::disclosure_expansion(&section.header_response);
                    }
                }
                self.snapshot.show_quiet(ui);
                ui.ctx().request_repaint_after(Duration::from_secs(1));
            }
            "network-usage" => {
                ui.label("Local interface counters only; no network service is contacted.");
                ui.checkbox(&mut self.virtuals, "Include virtual / loopback interfaces");
                if self.last_network.elapsed() >= Duration::from_secs(1) && !self.traffic.running()
                {
                    self.last_network = Instant::now();
                    let network = self.network.clone();
                    self.traffic.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        let mut guard = runtime::lock(&network)?;
                        Ok(guard.get_or_insert_with(NetworkMonitor::default).sample())
                    });
                }
                if let Some(interfaces) = &self.traffic.result {
                    let rows: Vec<_> = interfaces
                        .iter()
                        .filter(|i| self.virtuals || !i.virtual_interface)
                        .collect();
                    if rows.is_empty() {
                        ui.label("No matching network interfaces are available.");
                    }
                    for row in rows {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.strong(&row.name);
                            ui.label(format!(
                                "Download {}/s · upload {}/s",
                                bytes(row.download_per_second.max(0.0) as u64),
                                bytes(row.upload_per_second.max(0.0) as u64)
                            ));
                            ui.label(format!(
                                "Session received {} · sent {}",
                                bytes(row.received),
                                bytes(row.transmitted)
                            ));
                        });
                    }
                }
                self.traffic.show_quiet(ui);
                ui.ctx().request_repaint_after(Duration::from_secs(1));
            }
            "cpu-temperature" => {
                ui.label("CPU sensors refresh every two seconds while this view is open. Many systems expose none; no driver or service is installed.");
                let busy = self.sensors.running();
                let refresh = ui
                    .add_enabled(!busy, egui::Button::new("Refresh temperature sensors"))
                    .clicked();
                let now_ms = self
                    .sampling_origin
                    .elapsed()
                    .as_millis()
                    .min(u128::from(u64::MAX)) as u64;
                if !busy
                    && (refresh
                        || cpu_temperature::sample_due(true, busy, self.last_sensor_ms, now_ms))
                    && self.sensors.start(ui.ctx(), |cancel, _| {
                        cancel.check()?;
                        Ok(cpu_temperature::sample())
                    })
                {
                    self.last_sensor_ms = Some(now_ms);
                }
                if let Some(sensors) = &self.sensors.result {
                    if sensors.is_empty() {
                        ui.label("No CPU temperature sensors were exposed by this system.");
                    }
                    for s in sensors {
                        ui.label(format!(
                            "{}: {:.1} °C{}",
                            s.label,
                            s.temperature,
                            s.critical
                                .map_or(String::new(), |c| format!(" · critical {c:.1} °C"))
                        ));
                    }
                }
                self.sensors.show_quiet(ui);
                ui.ctx().request_repaint_after(Duration::from_secs(2));
            }
            _ => return false,
        }
        true
    }
    pub fn shutdown(&mut self) {
        self.snapshot.shutdown();
        self.traffic.shutdown();
        self.sensors.shutdown();
    }
    pub fn leave_tool(&mut self, id: &str) {
        match id {
            "system-monitor" => self.snapshot.abandon(),
            "network-usage" => self.traffic.abandon(),
            "cpu-temperature" => self.sensors.abandon(),
            _ => {}
        }
    }
}
pub fn bytes(value: u64) -> String {
    let value = value as f64;
    if value >= 1073741824.0 {
        format!("{:.2} GiB", value / 1073741824.0)
    } else if value >= 1048576.0 {
        format!("{:.1} MiB", value / 1048576.0)
    } else if value >= 1024.0 {
        format!("{:.1} KiB", value / 1024.0)
    } else {
        format!("{value:.0} B")
    }
}
