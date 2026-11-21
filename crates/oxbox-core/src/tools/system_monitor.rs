//! Tool: **System Monitor** (`system-monitor`). Snapshot collection is opt-in per poll.
use serde::Serialize;
use sysinfo::{Disks, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub available: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub cpu_percent: f32,
    pub cores: Vec<f32>,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disks: Vec<DiskInfo>,
    pub processes_cpu: Vec<ProcessInfo>,
    pub processes_memory: Vec<ProcessInfo>,
}

pub struct Monitor {
    system: System,
    disks: Disks,
}
impl Default for Monitor {
    fn default() -> Self {
        Self {
            system: System::new(),
            disks: Disks::new(),
        }
    }
}
fn percentage(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}
impl Monitor {
    pub fn snapshot(&mut self) -> Snapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.disks.refresh(true);
        let cores = self.system.cpus().len().max(1) as f32;
        let mut processes_cpu: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, p)| ProcessInfo {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                cpu_percent: percentage(p.cpu_usage() / cores),
                memory: p.memory(),
            })
            .collect();
        let mut processes_memory = processes_cpu.clone();
        processes_cpu.sort_by(|a, b| {
            b.cpu_percent
                .total_cmp(&a.cpu_percent)
                .then_with(|| a.pid.cmp(&b.pid))
        });
        processes_memory.sort_by(|a, b| b.memory.cmp(&a.memory).then_with(|| a.pid.cmp(&b.pid)));
        processes_cpu.truncate(10);
        processes_memory.truncate(10);
        Snapshot {
            cpu_percent: percentage(self.system.global_cpu_usage()),
            cores: self
                .system
                .cpus()
                .iter()
                .map(|c| percentage(c.cpu_usage()))
                .collect(),
            memory_total: self.system.total_memory(),
            memory_used: self.system.used_memory().min(self.system.total_memory()),
            swap_total: self.system.total_swap(),
            swap_used: self.system.used_swap().min(self.system.total_swap()),
            disks: self
                .disks
                .iter()
                .map(|d| DiskInfo {
                    name: d.name().to_string_lossy().into_owned(),
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    total: d.total_space(),
                    available: d.available_space().min(d.total_space()),
                })
                .collect(),
            processes_cpu,
            processes_memory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_has_bounded_values_and_serializes() {
        let mut monitor = Monitor::default();
        let s = monitor.snapshot();
        assert!(s.cpu_percent.is_finite());
        assert!((0.0..=100.0).contains(&s.cpu_percent));
        assert!(s.memory_used <= s.memory_total);
        assert!(s.swap_used <= s.swap_total);
        assert!(s.processes_cpu.len() <= 10);
        assert!(s.disks.iter().all(|d| d.available <= d.total));
        assert!(serde_json::to_string(&s).unwrap().contains("cpuPercent"));
    }
}
