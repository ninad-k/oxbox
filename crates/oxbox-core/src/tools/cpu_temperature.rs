//! Read available CPU temperature sensors without installing a driver.
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sensor {
    pub label: String,
    pub temperature: f32,
    pub critical: Option<f32>,
}
pub fn valid_temperature(value: Option<f32>) -> Option<f32> {
    value.filter(|v| v.is_finite() && (-40.0..=200.0).contains(v))
}
pub fn is_cpu_sensor(label: &str) -> bool {
    let label = label.to_lowercase();
    [
        "cpu", "core ", "package", "tctl", "tdie", "k10temp", "zenpower",
    ]
    .iter()
    .any(|part| label.contains(part))
}
pub fn sample() -> Vec<Sensor> {
    sysinfo::Components::new_with_refreshed_list()
        .iter()
        .filter(|c| is_cpu_sensor(c.label()))
        .filter_map(|c| {
            valid_temperature(c.temperature()).map(|temperature| Sensor {
                label: c.label().into(),
                temperature,
                critical: valid_temperature(c.critical()),
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_nonfinite_and_implausible_readings() {
        assert_eq!(valid_temperature(Some(55.0)), Some(55.0));
        assert_eq!(valid_temperature(None), None);
        assert_eq!(valid_temperature(Some(f32::NAN)), None);
        assert_eq!(valid_temperature(Some(2000.0)), None);
        assert!(is_cpu_sensor("Package id 0"));
        assert!(!is_cpu_sensor("NVMe Composite"));
    }
}
