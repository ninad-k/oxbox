//! Tool: **Alarm**. Native callers inject UTC seconds; local recurrences use bundled IANA rules.
use super::time_support;
use crate::{CoreError, CoreResult};
use jiff::{
    civil::{DateTime, Weekday},
    tz::{AmbiguousOffset, TimeZone},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ambiguity {
    Earlier,
    Later,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Recurrence {
    Once,
    Daily,
    Weekdays,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmInput {
    pub label: String,
    pub local_date_time: String,
    pub zone: String,
    pub recurrence: Recurrence,
    pub ambiguity: Ambiguity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub id: u64,
    pub input: AlarmInput,
    pub next_at: i64,
    pub enabled: bool,
    pub ringing: bool,
    pub last_fired: Option<i64>,
}
#[derive(Debug, Clone, Default)]
pub struct AlarmBook {
    pub items: Vec<Alarm>,
}
pub fn validate_label(label: &str) -> CoreResult<()> {
    if label.trim().is_empty() || label.len() > 200 || label.chars().any(char::is_control) {
        return Err(CoreError::InvalidInput(
            "Use a name from 1 to 200 bytes without control characters.".into(),
        ));
    }
    Ok(())
}
fn local(value: &str) -> CoreResult<i64> {
    if value.len() > 25 {
        return Err(CoreError::InvalidInput(
            "Invalid local date and time.".into(),
        ));
    }
    let invalid = || CoreError::InvalidInput("Use a valid local date and time.".into());
    // The old numeric directives skipped Unicode whitespace before each
    // number, and accepted explicitly signed, padded years. Normalize only
    // those positions; literal separators and trailing text remain strict.
    let value = value.trim_start();
    let (negative, signed, value) = if let Some(value) = value.strip_prefix('-') {
        (true, true, value)
    } else if let Some(value) = value.strip_prefix('+') {
        (false, true, value)
    } else {
        (false, false, value)
    };
    let (year, rest) = value.split_once('-').ok_or_else(invalid)?;
    if year.is_empty()
        || (!signed && year.len() > 4)
        || !year.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let year = year.parse::<i32>().map_err(|_| invalid())?;
    let year = if negative {
        year.checked_neg().ok_or_else(invalid)?
    } else {
        year
    };
    // Match the preceding parser's calendar range before the tool's stricter
    // saved-alarm year check, including its distinct error message.
    if !(-262_143..=262_142).contains(&year) {
        return Err(invalid());
    }
    let component = |value: &str| -> CoreResult<i8> {
        let value = value.trim_start();
        if value.is_empty() || value.len() > 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        value.parse().map_err(|_| invalid())
    };
    let (month, rest) = rest.split_once('-').ok_or_else(invalid)?;
    let (day, rest) = rest.split_once('T').ok_or_else(invalid)?;
    let (hour, rest) = rest.split_once(':').ok_or_else(invalid)?;
    let (minute, second) = match rest.split_once(':') {
        Some((minute, second)) => (component(minute)?, component(second)?),
        None => (component(rest)?, 0),
    };
    let calendar_year = if (1..=9999).contains(&year) {
        year
    } else {
        // Only calendar validity is needed for an out-of-tool-range year.
        // Gregorian dates repeat every400 years; Jiff remains the validator.
        year.rem_euclid(400)
    };
    let canonical = format!(
        "{calendar_year:04}-{:02}-{:02}T{:02}:{minute:02}:{second:02}",
        component(month)?,
        component(day)?,
        component(hour)?,
    );
    let parsed = DateTime::strptime("%Y-%m-%dT%H:%M:%S", canonical).map_err(|_| invalid())?;
    if !(1..=9999).contains(&year) {
        return Err(CoreError::InvalidInput(
            "Choose a year between 1 and 9999.".into(),
        ));
    }
    Ok(time_support::wall_seconds(parsed))
}
fn timezone(zone: &str) -> CoreResult<TimeZone> {
    if zone.len() > 128 {
        return Err(CoreError::InvalidInput(
            "Time zone name is too long.".into(),
        ));
    }
    time_support::timezone(zone)
        .map_err(|_| CoreError::InvalidInput("Choose a valid IANA time zone.".into()))
}
fn resolve(date: i64, zone: &TimeZone, ambiguity: Ambiguity) -> CoreResult<i64> {
    let offset = match time_support::wall_offset(zone, date)? {
        AmbiguousOffset::Unambiguous { offset } => offset.seconds(),
        AmbiguousOffset::Fold { before, after } => match ambiguity {
            Ambiguity::Earlier => before.seconds().max(after.seconds()),
            Ambiguity::Later => before.seconds().min(after.seconds()),
        },
        AmbiguousOffset::Gap { .. } => return Err(CoreError::InvalidInput(
            "This local time does not exist because the clock moves forward. Choose another time."
                .into(),
        )),
    };
    date.checked_sub(i64::from(offset))
        .ok_or_else(|| CoreError::InvalidInput("Date exceeds the supported range.".into()))
}
pub fn resolve_local(value: &str, zone: &str, ambiguity: Ambiguity) -> CoreResult<i64> {
    resolve(local(value)?, &timezone(zone)?, ambiguity)
}
pub fn next_occurrence(input: &AlarmInput, after: i64) -> CoreResult<i64> {
    validate_label(&input.label)?;
    let requested = local(&input.local_date_time)?;
    let zone = timezone(&input.zone)?;
    if input.recurrence == Recurrence::Once {
        let at = resolve(requested, &zone, input.ambiguity)?;
        if at <= after {
            return Err(CoreError::InvalidInput(
                "A one-time alarm must be in the future.".into(),
            ));
        }
        return Ok(at);
    }
    if !time_support::valid_timestamp(after) {
        return Err(CoreError::InvalidInput("Invalid clock timestamp.".into()));
    }
    let offset = time_support::offset_info(&zone, after)?;
    let now = after
        .checked_add(i64::from(offset.seconds))
        .ok_or_else(|| CoreError::InvalidInput("Invalid clock timestamp.".into()))?;
    let start = now
        .div_euclid(time_support::DAY_SECONDS)
        .max(requested.div_euclid(time_support::DAY_SECONDS));
    for days in 0..9 {
        let date = start
            .checked_add(days)
            .ok_or_else(|| CoreError::InvalidInput("Date exceeds the supported range.".into()))?;
        if date < time_support::MIN_TIMESTAMP.div_euclid(time_support::DAY_SECONDS)
            || date > time_support::MAX_TIMESTAMP.div_euclid(time_support::DAY_SECONDS)
        {
            return Err(CoreError::InvalidInput(
                "Date exceeds the supported range.".into(),
            ));
        }
        let midnight = date
            .checked_mul(time_support::DAY_SECONDS)
            .ok_or_else(|| CoreError::InvalidInput("Date exceeds the supported range.".into()))?;
        if input.recurrence == Recurrence::Weekdays
            && matches!(
                time_support::wall_parts(midnight)?.weekday,
                Weekday::Saturday | Weekday::Sunday
            )
        {
            continue;
        }
        let candidate = midnight
            .checked_add(requested.rem_euclid(time_support::DAY_SECONDS))
            .ok_or_else(|| CoreError::InvalidInput("Date exceeds the supported range.".into()))?;
        if let Ok(at) = resolve(candidate, &zone, input.ambiguity)
            && at > after
        {
            return Ok(at);
        }
    }
    Err(CoreError::InvalidInput(
        "No valid recurrence was found in the next nine days.".into(),
    ))
}
impl AlarmBook {
    pub fn restore(items: Vec<Alarm>) -> CoreResult<Self> {
        if items.len() > 100 {
            return Err(CoreError::InvalidInput(
                "At most 100 alarms are allowed.".into(),
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for item in &items {
            validate_label(&item.input.label)?;
            local(&item.input.local_date_time)?;
            timezone(&item.input.zone)?;
            if item.id == 0
                || item.id > 9_007_199_254_740_991
                || !ids.insert(item.id)
                || !time_support::valid_timestamp(item.next_at)
            {
                return Err(CoreError::InvalidInput(
                    "Alarm storage contains an invalid identifier or timestamp.".into(),
                ));
            }
        }
        Ok(Self { items })
    }
    pub fn add(&mut self, input: AlarmInput, now: i64) -> CoreResult<u64> {
        if self.items.len() >= 100 {
            return Err(CoreError::InvalidInput(
                "At most 100 alarms are allowed.".into(),
            ));
        }
        let next_at = next_occurrence(&input, now)?;
        let id = self
            .items
            .iter()
            .map(|a| a.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .filter(|id| *id <= 9_007_199_254_740_991)
            .ok_or_else(|| CoreError::InvalidInput("Alarm identifier range exhausted.".into()))?;
        self.items.push(Alarm {
            id,
            input,
            next_at,
            enabled: true,
            ringing: false,
            last_fired: None,
        });
        Ok(id)
    }
    fn item(&mut self, id: u64) -> CoreResult<&mut Alarm> {
        self.items
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| CoreError::InvalidInput("Alarm was not found.".into()))
    }
    pub fn remove(&mut self, id: u64) -> CoreResult<()> {
        self.item(id)?;
        self.items.retain(|a| a.id != id);
        Ok(())
    }
    pub fn dismiss(&mut self, id: u64) -> CoreResult<()> {
        self.item(id)?.ringing = false;
        Ok(())
    }
    pub fn set_enabled(&mut self, id: u64, enabled: bool, now: i64) -> CoreResult<()> {
        let item = self.item(id)?;
        if enabled {
            item.next_at = next_occurrence(&item.input, now)?;
        }
        item.enabled = enabled;
        item.ringing = false;
        Ok(())
    }
    pub fn snooze(&mut self, id: u64, now: i64, minutes: u64) -> CoreResult<()> {
        if minutes == 0 || minutes > 1440 {
            return Err(CoreError::InvalidInput(
                "Snooze must be between 1 and 1440 minutes.".into(),
            ));
        }
        let item = self.item(id)?;
        item.next_at = now
            .checked_add(minutes as i64 * 60)
            .ok_or_else(|| CoreError::InvalidInput("Snooze is out of range.".into()))?;
        item.enabled = true;
        item.ringing = false;
        Ok(())
    }
    pub fn tick(&mut self, now: i64) -> Vec<Alarm> {
        let mut fired = Vec::new();
        for item in &mut self.items {
            if !item.enabled || item.next_at > now {
                continue;
            }
            item.ringing = true;
            item.last_fired = Some(now);
            fired.push(item.clone());
            if item.input.recurrence == Recurrence::Once {
                item.enabled = false;
            } else {
                match next_occurrence(&item.input, now) {
                    Ok(at) => item.next_at = at,
                    Err(_) => item.enabled = false,
                }
            }
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alarm_legacy_grammar_preserves_numeric_whitespace_and_signed_padding() {
        // Captured from the original Chrono parser, including the UTF-8 byte cap.
        for (value, expected) in [
            ("\u{2003}2024-1-2T3:4", 1_704_164_640),
            ("2024-\u{2003}1-2T3:4", 1_704_164_640),
            ("2024-1-\u{2003}2T3:4", 1_704_164_640),
            ("2024-1-2T\u{2003}3:4", 1_704_164_640),
            ("2024-1-2T3:\u{2003}4", 1_704_164_640),
            ("2024-1-2T3:4:\u{2003}5", 1_704_164_645),
            ("+00002024-01-02T03:04", 1_704_164_640),
            (" +00002024-01-02T03:04", 1_704_164_640),
            ("1-1-1T1:2", -62_135_593_080),
            ("2024-1-2T3:\t4", 1_704_164_640),
            ("2024-1-2T3:\n4", 1_704_164_640),
        ] {
            assert_eq!(
                resolve_local(value, "UTC", Ambiguity::Earlier).unwrap(),
                expected,
                "{value:?}"
            );
        }
        for value in [
            "2024-1-2T3:4 ",
            "2024 -1-2T3:4",
            "2024-001-2T3:4",
            "2024-1-002T3:4",
            "2024-1-2T003:4",
            "2024-1-2T3:004",
            "2024-1-2T3:4:005",
            "２０２４-1-2T3:4",
            "2024-1-2t3:4",
            "2024-2-30T3:4",
            "2024-1-2T24:00",
        ] {
            assert!(
                matches!(
                    resolve_local(value, "UTC", Ambiguity::Earlier),
                    Err(CoreError::InvalidInput(message)) if message == "Use a valid local date and time."
                ),
                "{value:?}"
            );
        }
        for value in ["0000-1-2T3:4", "+10000-1-2T3:4", "-00002024-1-2T3:4"] {
            assert!(
                matches!(
                    resolve_local(value, "UTC", Ambiguity::Earlier),
                    Err(CoreError::InvalidInput(message)) if message == "Choose a year between 1 and 9999."
                ),
                "{value:?}"
            );
        }
        assert!(matches!(
            resolve_local("\u{2003}\u{2003}\u{2003}2024-01-02T03:04:05", "UTC", Ambiguity::Earlier),
            Err(CoreError::InvalidInput(message)) if message == "Invalid local date and time."
        ));
    }
    #[test]
    fn jiff_future_alarm_uses_recurring_dst_instead_of_the_finite_2099_tail() {
        let epoch = jiff::civil::date(1970, 1, 1).at(0, 0, 0, 0);
        let after = jiff::civil::date(2200, 7, 1)
            .at(12, 0, 0, 0)
            .duration_since(epoch)
            .as_secs();
        let input = AlarmInput {
            label: "Generated future recurrence".into(),
            local_date_time: "2200-07-01T09:00".into(),
            zone: "America/New_York".into(),
            recurrence: Recurrence::Daily,
            ambiguity: Ambiguity::Earlier,
        };
        assert_eq!(next_occurrence(&input, after).unwrap(), after + 3600);
    }
    #[test]
    fn timezone_resolution_rejects_gap_and_selects_fold() {
        assert!(resolve_local("2024-03-10T02:30", "America/New_York", Ambiguity::Earlier).is_err());
        let earlier =
            resolve_local("2024-11-03T01:30", "America/New_York", Ambiguity::Earlier).unwrap();
        let later =
            resolve_local("2024-11-03T01:30", "America/New_York", Ambiguity::Later).unwrap();
        assert_eq!(later - earlier, 3600);
        assert!(resolve_local("bad", "UTC", Ambiguity::Earlier).is_err());
        assert!(resolve_local("2030-01-01T10:00", "Not/AZone", Ambiguity::Earlier).is_err());
    }
    #[test]
    fn fake_clock_fires_once_snoozes_and_dismisses() {
        let mut book = AlarmBook::default();
        let id = book
            .add(
                AlarmInput {
                    label: "Tea".into(),
                    local_date_time: "2030-01-01T10:00".into(),
                    zone: "UTC".into(),
                    recurrence: Recurrence::Once,
                    ambiguity: Ambiguity::Earlier,
                },
                0,
            )
            .unwrap();
        let due = book.items[0].next_at;
        assert!(book.tick(due - 1).is_empty());
        assert_eq!(book.tick(due).len(), 1);
        assert!(book.tick(due + 1).is_empty());
        book.snooze(id, due, 5).unwrap();
        assert!(book.tick(due + 299).is_empty());
        assert_eq!(book.tick(due + 300).len(), 1);
        book.dismiss(id).unwrap();
        assert!(!book.items[0].ringing);
        assert!(book.snooze(999, due, 5).is_err());
    }
    #[test]
    fn weekdays_skip_weekends_and_daily_skips_dst_gaps() {
        let mut book = AlarmBook::default();
        book.add(
            AlarmInput {
                label: "Work".into(),
                local_date_time: "2030-01-04T10:00".into(),
                zone: "UTC".into(),
                recurrence: Recurrence::Weekdays,
                ambiguity: Ambiguity::Earlier,
            },
            0,
        )
        .unwrap();
        let friday = resolve_local("2030-01-04T10:00", "UTC", Ambiguity::Earlier).unwrap();
        assert_eq!(book.tick(friday).len(), 1);
        assert_eq!(
            book.items[0].next_at,
            resolve_local("2030-01-07T10:00", "UTC", Ambiguity::Earlier).unwrap()
        );
        let input = AlarmInput {
            label: "Daily".into(),
            local_date_time: "2024-03-09T02:30".into(),
            zone: "America/New_York".into(),
            recurrence: Recurrence::Daily,
            ambiguity: Ambiguity::Earlier,
        };
        let after =
            resolve_local("2024-03-09T03:00", "America/New_York", Ambiguity::Earlier).unwrap();
        assert_eq!(
            next_occurrence(&input, after).unwrap(),
            resolve_local("2024-03-11T02:30", "America/New_York", Ambiguity::Earlier).unwrap()
        );
    }
    #[test]
    fn restored_fired_alarm_does_not_notify_twice_and_rejects_duplicate_ids() {
        let mut book = AlarmBook::default();
        book.add(
            AlarmInput {
                label: "One shot".into(),
                local_date_time: "2030-01-01T00:00".into(),
                zone: "UTC".into(),
                recurrence: Recurrence::Once,
                ambiguity: Ambiguity::Earlier,
            },
            0,
        )
        .unwrap();
        let due = book.items[0].next_at;
        assert_eq!(book.tick(due).len(), 1);
        let encoded = serde_json::to_string(&book.items).unwrap();
        let mut restored = AlarmBook::restore(serde_json::from_str(&encoded).unwrap()).unwrap();
        assert!(restored.tick(due + 60).is_empty());
        assert!(restored.items[0].ringing);
        assert!(
            AlarmBook::restore(vec![restored.items[0].clone(), restored.items[0].clone()]).is_err()
        );
        assert!(restored.set_enabled(1, true, due + 60).is_err());
        assert!(resolve_local("+262140-01-01T00:00", "UTC", Ambiguity::Earlier).is_err());
    }
}
