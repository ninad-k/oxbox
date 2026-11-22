//! Checked civil-time compatibility and the maintained embedded IANA database.
//!
//! A Gregorian400-year cycle is146097 days exactly. It is used to format wide
//! epoch-second values through Jiff's bounded civil calendar without narrowing
//! the preceding public timestamp range. Ancient instants retain the IANA first
//! offset. Beyond Jiff's timestamp range, future annual POSIX rules are evaluated
//! in a safe future cycle, rather than freezing Chrono-TZ's finite2099 tail.
use crate::{CoreError, CoreResult};
use jiff::{
    Timestamp,
    civil::{DateTime, Weekday},
    tz::{AmbiguousOffset, TimeZone, TimeZoneDatabase},
};
use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const MIN_TIMESTAMP: i64 = -8_334_601_228_800;
pub(crate) const MAX_TIMESTAMP: i64 = 8_210_266_876_799;
pub(crate) const DAY_SECONDS: i64 = 86_400;
const CYCLE_SECONDS: i64 = 146_097 * DAY_SECONDS;
const ANCHOR_2000: i64 = 946_684_800;
const ANCHOR_9200: i64 = ANCHOR_2000 + 18 * CYCLE_SECONDS;

fn invalid() -> CoreError {
    CoreError::InvalidInput("invalid timestamp".into())
}
pub(crate) fn valid_timestamp(value: i64) -> bool {
    (MIN_TIMESTAMP..=MAX_TIMESTAMP).contains(&value)
}
pub(crate) fn zones() -> Vec<&'static str> {
    include_str!("world_time_zones.txt").lines().collect()
}
pub(crate) fn timezone(name: &str) -> CoreResult<TimeZone> {
    if !include_str!("world_time_zones.txt")
        .lines()
        .any(|candidate| candidate == name)
    {
        return Err(CoreError::InvalidInput("unknown IANA time zone".into()));
    }
    static DATABASE: OnceLock<TimeZoneDatabase> = OnceLock::new();
    DATABASE
        .get_or_init(TimeZoneDatabase::bundled)
        .get(name)
        .map_err(|_| CoreError::InvalidInput("unknown IANA time zone".into()))
}
fn representative(seconds: i64, anchor: i64) -> CoreResult<(DateTime, i64)> {
    let cycles = seconds
        .checked_sub(anchor)
        .ok_or_else(invalid)?
        .div_euclid(CYCLE_SECONDS);
    let shift = cycles.checked_mul(CYCLE_SECONDS).ok_or_else(invalid)?;
    let shifted = seconds.checked_sub(shift).ok_or_else(invalid)?;
    let timestamp = Timestamp::from_second(shifted).map_err(|_| invalid())?;
    Ok((TimeZone::UTC.to_datetime(timestamp), cycles))
}
pub(crate) struct CivilParts {
    pub year: i32,
    pub month: i8,
    pub day: i8,
    pub hour: i8,
    pub minute: i8,
    pub second: i8,
    pub weekday: Weekday,
}
pub(crate) fn wall_parts(seconds: i64) -> CoreResult<CivilParts> {
    let (value, cycles) = representative(seconds, ANCHOR_2000)?;
    let year = cycles
        .checked_mul(400)
        .and_then(|cycles| cycles.checked_add(i64::from(value.year())))
        .and_then(|year| i32::try_from(year).ok())
        .ok_or_else(invalid)?;
    Ok(CivilParts {
        year,
        month: value.month(),
        day: value.day(),
        hour: value.hour(),
        minute: value.minute(),
        second: value.second(),
        weekday: value.weekday(),
    })
}
pub(crate) fn date_text(parts: &CivilParts) -> String {
    let year = if parts.year > 9999 {
        format!("+{}", parts.year)
    } else if parts.year < 0 {
        format!("-{:04}", parts.year.unsigned_abs())
    } else {
        format!("{:04}", parts.year)
    };
    format!("{year}-{:02}-{:02}", parts.month, parts.day)
}
pub(crate) fn time_text(parts: &CivilParts) -> String {
    format!("{:02}:{:02}:{:02}", parts.hour, parts.minute, parts.second)
}
pub(crate) fn wall_seconds(value: DateTime) -> i64 {
    value
        .duration_since(jiff::civil::date(1970, 1, 1).at(0, 0, 0, 0))
        .as_secs()
}
pub(crate) fn wall_offset(zone: &TimeZone, seconds: i64) -> CoreResult<AmbiguousOffset> {
    let parts = wall_parts(seconds)?;
    if parts.year < -9999 {
        return Ok(AmbiguousOffset::Unambiguous {
            offset: zone.to_offset(Timestamp::MIN),
        });
    }
    let local = if parts.year > 9999 {
        representative(seconds, ANCHOR_9200)?.0
    } else {
        DateTime::new(
            parts.year as i16,
            parts.month,
            parts.day,
            parts.hour,
            parts.minute,
            parts.second,
            0,
        )
        .map_err(|_| invalid())?
    };
    Ok(zone.to_ambiguous_timestamp(local).offset())
}
pub(crate) struct OffsetInfo {
    pub seconds: i32,
    pub abbreviation: String,
}
pub(crate) fn offset_info(zone: &TimeZone, seconds: i64) -> CoreResult<OffsetInfo> {
    let instant = match Timestamp::from_second(seconds) {
        Ok(value) => value,
        Err(_) if seconds < 0 => Timestamp::MIN,
        Err(_) => {
            let (value, _) = representative(seconds, ANCHOR_9200)?;
            Timestamp::from_second(wall_seconds(value)).map_err(|_| invalid())?
        }
    };
    let info = zone.to_offset_info(instant);
    Ok(OffsetInfo {
        seconds: info.offset().seconds(),
        abbreviation: info.abbreviation().into(),
    })
}
/// Current Unix seconds, with the same floor semantics before the epoch.
pub fn now_timestamp() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(error) => {
            let duration = error.duration();
            i64::try_from(duration.as_secs())
                .ok()
                .and_then(|seconds| seconds.checked_add(i64::from(duration.subsec_nanos() != 0)))
                .and_then(i64::checked_neg)
                .unwrap_or(i64::MIN)
        }
    }
}
/// Read the local OS-zone offset without changing any clock or timezone setting.
pub fn local_offset_seconds(timestamp: i64) -> CoreResult<i32> {
    if !valid_timestamp(timestamp) {
        return Err(invalid());
    }
    Ok(offset_info(&TimeZone::system(), timestamp)?.seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_time_support_cycles_preserve_leap_day_weekday_and_reject_overflow() {
        let reference = wall_seconds(jiff::civil::date(2000, 2, 29).at(12, 34, 56, 0));
        for cycles in [-600, -5, -1, 0, 1, 19, 600] {
            let parts = wall_parts(reference + cycles * CYCLE_SECONDS).unwrap();
            assert_eq!(parts.year, 2000 + 400 * cycles as i32);
            assert_eq!(
                (
                    parts.month,
                    parts.day,
                    parts.hour,
                    parts.minute,
                    parts.second
                ),
                (2, 29, 12, 34, 56)
            );
            assert_eq!(parts.weekday, Weekday::Tuesday);
        }
        assert!(wall_parts(i64::MIN).is_err());
        assert!(wall_parts(i64::MAX).is_err());
        assert!(valid_timestamp(MIN_TIMESTAMP) && valid_timestamp(MAX_TIMESTAMP));
        assert!(!valid_timestamp(MIN_TIMESTAMP - 1) && !valid_timestamp(MAX_TIMESTAMP + 1));
    }

    #[test]
    fn checked_time_support_ancient_and_future_offsets_use_distinct_policies() {
        let zone = timezone("America/New_York").unwrap();
        let ancient = wall_seconds(jiff::civil::date(-9600, 7, 1).at(12, 0, 0, 0)) - CYCLE_SECONDS;
        let future = wall_seconds(jiff::civil::date(9600, 7, 1).at(12, 0, 0, 0)) + CYCLE_SECONDS;
        for (wall, expected) in [(ancient, -17_762), (future, -14_400)] {
            let AmbiguousOffset::Unambiguous { offset } = wall_offset(&zone, wall).unwrap() else {
                panic!("generated July wall time must be unambiguous");
            };
            assert_eq!(offset.seconds(), expected);
            assert_eq!(offset_info(&zone, wall).unwrap().seconds, expected);
        }
    }

    #[test]
    fn checked_time_support_alarm_end_year_survives_the_timestamp_offset_margin() {
        let wall = wall_seconds(jiff::civil::date(9999, 12, 31).at(23, 59, 59, 0));
        assert!(
            Timestamp::from_second(wall).is_err(),
            "fixture must cross Jiff's instant margin"
        );
        let instant = super::super::alarm::resolve_local(
            "9999-12-31T23:59:59",
            "America/New_York",
            super::super::alarm::Ambiguity::Earlier,
        )
        .unwrap();
        assert_eq!(instant, wall + 18_000);
        let clock = super::super::world_time::clock_at("America/New_York", instant).unwrap();
        assert_eq!(
            (clock.date.as_str(), clock.time.as_str()),
            ("9999-12-31", "23:59:59")
        );
    }
}
