//! Tool: **Network Usage** (`network-usage`). Reads local interface counters only.
use serde::Serialize;
use std::{collections::HashMap, time::Instant};

pub fn bytes_per_second(previous: u64, current: u64, elapsed: f64) -> f64 {
    if !elapsed.is_finite() || elapsed <= 0.0 {
        return 0.0;
    }
    current.saturating_sub(previous) as f64 / elapsed
}

pub fn is_virtual(name: &str) -> bool {
    let name = name.to_lowercase();
    name == "lo"
        || [
            "loopback",
            "veth",
            "docker",
            "br-",
            "virbr",
            "utun",
            "tun",
            "tap",
            "vmnet",
            "vbox",
            "tailscale",
            "zerotier",
        ]
        .iter()
        .any(|p| name.starts_with(p))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceUsage {
    pub name: String,
    pub download_per_second: f64,
    pub upload_per_second: f64,
    pub received: u64,
    pub transmitted: u64,
    pub virtual_interface: bool,
}

struct Counters {
    received: u64,
    transmitted: u64,
    session_received: u64,
    session_transmitted: u64,
}

pub struct NetworkMonitor {
    networks: sysinfo::Networks,
    previous: HashMap<String, Counters>,
    sampled: Instant,
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let previous = networks
            .iter()
            .map(|(name, n)| {
                (
                    name.clone(),
                    Counters {
                        received: n.total_received(),
                        transmitted: n.total_transmitted(),
                        session_received: 0,
                        session_transmitted: 0,
                    },
                )
            })
            .collect();
        Self {
            networks,
            previous,
            sampled: Instant::now(),
        }
    }
}

impl NetworkMonitor {
    pub fn sample(&mut self) -> Vec<InterfaceUsage> {
        self.networks.refresh(true);
        let now = Instant::now();
        let elapsed = now.duration_since(self.sampled).as_secs_f64();
        self.sampled = now;
        let mut result = Vec::new();
        for (name, data) in &self.networks {
            let received = data.total_received();
            let transmitted = data.total_transmitted();
            let old = self.previous.entry(name.clone()).or_insert(Counters {
                received,
                transmitted,
                session_received: 0,
                session_transmitted: 0,
            });
            let download_per_second = bytes_per_second(old.received, received, elapsed);
            let upload_per_second = bytes_per_second(old.transmitted, transmitted, elapsed);
            old.session_received = old
                .session_received
                .saturating_add(received.saturating_sub(old.received));
            old.session_transmitted = old
                .session_transmitted
                .saturating_add(transmitted.saturating_sub(old.transmitted));
            result.push(InterfaceUsage {
                name: name.clone(),
                download_per_second,
                upload_per_second,
                received: old.session_received,
                transmitted: old.session_transmitted,
                virtual_interface: is_virtual(name),
            });
            old.received = received;
            old.transmitted = transmitted;
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_cover_normal_reset_and_zero_time() {
        assert_eq!(bytes_per_second(100, 300, 2.0), 100.0);
        assert_eq!(bytes_per_second(300, 10, 1.0), 0.0);
        assert_eq!(bytes_per_second(0, 100, 0.0), 0.0);
        assert_eq!(bytes_per_second(0, 100, f64::NAN), 0.0);
    }
    #[test]
    fn filters_virtual_interfaces() {
        for name in [
            "lo",
            "Loopback Pseudo-Interface 1",
            "vEthernet (WSL)",
            "docker0",
            "utun2",
        ] {
            assert!(is_virtual(name), "{name}");
        }
        assert!(!is_virtual("Wi-Fi"));
        assert!(!is_virtual("Ethernet"));
    }
}
