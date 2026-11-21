//! Tool: **Alarm**. Native callers inject UTC seconds; local recurrences use bundled IANA rules.
use crate::{CoreError, CoreResult};
use chrono::{DateTime, Datelike, Days, LocalResult, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
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
fn local(value: &str) -> CoreResult<NaiveDateTime> {
    if value.len() > 25 {
        return Err(CoreError::InvalidInput(
            "Invalid local date and time.".into(),
        ));
    }
    let parsed = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| CoreError::InvalidInput("Use a valid local date and time.".into()))?;
    if !(1..=9999).contains(&parsed.year()) {
        return Err(CoreError::InvalidInput(
            "Choose a year between 1 and 9999.".into(),
        ));
    }
    Ok(parsed)
}
fn timezone(zone: &str) -> CoreResult<Tz> {
    if zone.len() > 128 {
        return Err(CoreError::InvalidInput(
            "Time zone name is too long.".into(),
        ));
    }
    zone.parse()
        .map_err(|_| CoreError::InvalidInput("Choose a valid IANA time zone.".into()))
}
fn resolve(date: NaiveDateTime, zone: Tz, ambiguity: Ambiguity) -> CoreResult<i64> {
    match zone.from_local_datetime(&date) {
        LocalResult::Single(value) => Ok(value.timestamp()),
        LocalResult::Ambiguous(earlier, later) => Ok(match ambiguity {
            Ambiguity::Earlier => earlier.timestamp().min(later.timestamp()),
            Ambiguity::Later => earlier.timestamp().max(later.timestamp()),
        }),
        LocalResult::None => Err(CoreError::InvalidInput(
            "This local time does not exist because the clock moves forward. Choose another time."
                .into(),
        )),
    }
}
pub fn resolve_local(value: &str, zone: &str, ambiguity: Ambiguity) -> CoreResult<i64> {
    resolve(local(value)?, timezone(zone)?, ambiguity)
}
pub fn next_occurrence(input: &AlarmInput, after: i64) -> CoreResult<i64> {
    validate_label(&input.label)?;
    let requested = local(&input.local_date_time)?;
    let zone = timezone(&input.zone)?;
    if input.recurrence == Recurrence::Once {
        let at = resolve(requested, zone, input.ambiguity)?;
        if at <= after {
            return Err(CoreError::InvalidInput(
                "A one-time alarm must be in the future.".into(),
            ));
        }
        return Ok(at);
    }
    let now = DateTime::<Utc>::from_timestamp(after, 0)
        .ok_or_else(|| CoreError::InvalidInput("Invalid clock timestamp.".into()))?
        .with_timezone(&zone);
    let start = now.date_naive().max(requested.date());
    for days in 0..9 {
        let date = start
            .checked_add_days(Days::new(days))
            .ok_or_else(|| CoreError::InvalidInput("Date exceeds the supported range.".into()))?;
        if input.recurrence == Recurrence::Weekdays
            && matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
        {
            continue;
        }
        if let Ok(at) = resolve(date.and_time(requested.time()), zone, input.ambiguity)
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
                || DateTime::<Utc>::from_timestamp(item.next_at, 0).is_none()
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
