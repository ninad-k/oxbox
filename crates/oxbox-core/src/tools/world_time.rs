//! Tool: World Time. Convert UTC instants with bundled IANA time-zone rules.
use crate::{CoreError, CoreResult};
use chrono::{DateTime, Offset, Timelike, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};
use serde::Serialize;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clock {
    pub zone: String,
    pub date: String,
    pub time: String,
    pub abbreviation: String,
    pub offset_seconds: i32,
    pub daytime: bool,
}
pub fn zones() -> Vec<&'static str> {
    TZ_VARIANTS.iter().map(|zone| zone.name()).collect()
}
/// Search canonical IANA names and the readable city labels shown by the host.
pub fn search_zones(query: &str) -> Vec<&'static str> {
    let query = query.trim().to_lowercase().replace('_', " ");
    zones()
        .into_iter()
        .filter(|zone| zone.to_lowercase().replace('_', " ").contains(&query))
        .collect()
}
pub fn clock_at(zone: &str, timestamp: i64) -> CoreResult<Clock> {
    let zone: Tz = zone
        .parse()
        .map_err(|_| CoreError::InvalidInput("unknown IANA time zone".into()))?;
    let utc = DateTime::<Utc>::from_timestamp(timestamp, 0)
        .ok_or_else(|| CoreError::InvalidInput("invalid timestamp".into()))?;
    let local = utc.with_timezone(&zone);
    Ok(Clock {
        zone: zone.name().into(),
        date: local.format("%Y-%m-%d").to_string(),
        time: local.format("%H:%M:%S").to_string(),
        abbreviation: local.format("%Z").to_string(),
        offset_seconds: local.offset().fix().local_minus_utc(),
        daytime: (7..19).contains(&local.hour()),
    })
}
pub fn now(zone: &str) -> CoreResult<Clock> {
    clock_at(zone, Utc::now().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daylight_saving_boundary() {
        let before = clock_at("America/New_York", 1710053999).unwrap();
        let after = clock_at("America/New_York", 1710054000).unwrap();
        assert_eq!(before.offset_seconds, -18000);
        assert_eq!(after.offset_seconds, -14400);
        assert!(after.time.starts_with("03:00"));
    }
    #[test]
    fn zones_and_input_validation() {
        assert!(zones().contains(&"Asia/Dubai"));
        assert!(clock_at("Not/AZone", 0).is_err());
        assert!(clock_at("UTC", i64::MAX).is_err());
        assert_eq!(clock_at("UTC", 0).unwrap().date, "1970-01-01");
    }
    #[test]
    fn zone_search_accepts_canonical_iana_and_readable_city_queries() {
        for query in ["America/New_York", " new_york ", "NEW YORK", "new york"] {
            assert!(
                search_zones(query).contains(&"America/New_York"),
                "valid pasted IANA/city query had no matching clock: {query}"
            );
        }
        assert!(search_zones("uTc").contains(&"UTC"));
        assert!(search_zones("not a real time zone").is_empty());
        assert_eq!(search_zones(""), zones());
    }
}
