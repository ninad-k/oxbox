//! Tool: World Time. Convert UTC instants with bundled IANA time-zone rules.
use super::time_support;
use crate::{CoreError, CoreResult};
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
    time_support::zones()
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
    let timezone = time_support::timezone(zone)?;
    if !time_support::valid_timestamp(timestamp) {
        return Err(CoreError::InvalidInput("invalid timestamp".into()));
    }
    let offset = time_support::offset_info(&timezone, timestamp)?;
    let wall = timestamp
        .checked_add(i64::from(offset.seconds))
        .ok_or_else(|| CoreError::InvalidInput("invalid timestamp".into()))?;
    let local = time_support::wall_parts(wall)?;
    Ok(Clock {
        zone: zone.into(),
        date: time_support::date_text(&local),
        time: time_support::time_text(&local),
        abbreviation: offset.abbreviation,
        offset_seconds: offset.seconds,
        daytime: (7..19).contains(&local.hour),
    })
}
pub fn now(zone: &str) -> CoreResult<Clock> {
    clock_at(zone, time_support::now_timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jiff_future_world_clock_keeps_recurring_dst_and_checked_wide_dates() {
        let epoch = jiff::civil::date(1970, 1, 1).at(0, 0, 0, 0);
        let reference = jiff::civil::date(2200, 7, 1).at(12, 0, 0, 0);
        let timestamp = reference.duration_since(epoch).as_secs();
        for (timestamp, date) in [
            (timestamp, "2200-07-01"),
            (timestamp + 25 * 146_097 * 86_400, "+12200-07-01"),
        ] {
            let clock = clock_at("America/New_York", timestamp).unwrap();
            assert_eq!(
                clock.offset_seconds, -14_400,
                "future POSIX DST must not freeze at the last 2099 standard-time table entry"
            );
            assert_eq!(clock.abbreviation, "EDT");
            assert_eq!(clock.date, date);
            assert_eq!(clock.time, "08:00:00");
        }
    }
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
