//! Tool: **Date Countdown**. Fixed instants retain the chosen IANA zone and signed remaining time.
use super::alarm::{Ambiguity, resolve_local, validate_label};
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountdownInput {
    pub label: String,
    pub local_date_time: String,
    pub zone: String,
    pub notify: bool,
    pub ambiguity: Ambiguity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub id: u64,
    pub input: CountdownInput,
    pub target: i64,
    pub notified: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountdownSnapshot {
    #[serde(flatten)]
    pub item: Countdown,
    pub remaining_seconds: i64,
}
#[derive(Debug, Clone, Default)]
pub struct CountdownBook {
    pub items: Vec<Countdown>,
}
impl CountdownBook {
    pub fn restore(items: Vec<Countdown>) -> CoreResult<Self> {
        if items.len() > 100 {
            return Err(CoreError::InvalidInput(
                "At most 100 date countdowns are allowed.".into(),
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for item in &items {
            validate_label(&item.input.label)?;
            let target = resolve_local(
                &item.input.local_date_time,
                &item.input.zone,
                item.input.ambiguity,
            )?;
            if item.id == 0
                || item.id > 9_007_199_254_740_991
                || !ids.insert(item.id)
                || target != item.target
            {
                return Err(CoreError::InvalidInput(
                    "Countdown storage contains invalid identifiers or dates.".into(),
                ));
            }
        }
        Ok(Self { items })
    }
    pub fn add(&mut self, input: CountdownInput, now: i64) -> CoreResult<u64> {
        validate_label(&input.label)?;
        if self.items.len() >= 100 {
            return Err(CoreError::InvalidInput(
                "At most 100 date countdowns are allowed.".into(),
            ));
        }
        let target = resolve_local(&input.local_date_time, &input.zone, input.ambiguity)?;
        let id = self
            .items
            .iter()
            .map(|a| a.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .filter(|id| *id <= 9_007_199_254_740_991)
            .ok_or_else(|| {
                CoreError::InvalidInput("Countdown identifier range exhausted.".into())
            })?;
        // Past dates can be tracked; adding one does not produce a stale notification.
        self.items.push(Countdown {
            id,
            input,
            target,
            notified: target <= now,
        });
        Ok(id)
    }
    pub fn remove(&mut self, id: u64) -> CoreResult<()> {
        if !self.items.iter().any(|a| a.id == id) {
            return Err(CoreError::InvalidInput("Countdown was not found.".into()));
        }
        self.items.retain(|a| a.id != id);
        Ok(())
    }
    pub fn snapshot(&self, now: i64) -> Vec<CountdownSnapshot> {
        self.items
            .iter()
            .map(|item| CountdownSnapshot {
                item: item.clone(),
                remaining_seconds: item.target.saturating_sub(now),
            })
            .collect()
    }
    pub fn tick(&mut self, now: i64) -> Vec<Countdown> {
        let mut fired = Vec::new();
        for item in &mut self.items {
            if item.input.notify && !item.notified && item.target <= now {
                item.notified = true;
                fired.push(item.clone());
            }
        }
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn countdown_has_signed_time_and_notifies_once() {
        let mut book = CountdownBook::default();
        book.add(
            CountdownInput {
                label: "Launch".into(),
                local_date_time: "2030-01-01T00:00".into(),
                zone: "Asia/Dubai".into(),
                notify: true,
                ambiguity: crate::tools::alarm::Ambiguity::Earlier,
            },
            0,
        )
        .unwrap();
        let due = book.items[0].target;
        assert_eq!(book.snapshot(due - 90061)[0].remaining_seconds, 90061);
        assert!(book.tick(due - 1).is_empty());
        assert_eq!(book.tick(due).len(), 1);
        assert!(book.tick(due + 1).is_empty());
        assert_eq!(book.snapshot(due + 5)[0].remaining_seconds, -5);
    }
    #[test]
    fn invalid_dates_and_ids_are_errors() {
        let mut book = CountdownBook::default();
        assert!(
            book.add(
                CountdownInput {
                    label: "x".into(),
                    local_date_time: "bad".into(),
                    zone: "UTC".into(),
                    notify: false,
                    ambiguity: crate::tools::alarm::Ambiguity::Earlier
                },
                0
            )
            .is_err()
        );
        assert!(book.remove(7).is_err());
    }
    #[test]
    fn restored_delivered_countdown_and_new_past_date_do_not_notify() {
        let input = CountdownInput {
            label: "Date".into(),
            local_date_time: "2030-01-01T00:00".into(),
            zone: "UTC".into(),
            notify: true,
            ambiguity: crate::tools::alarm::Ambiguity::Earlier,
        };
        let mut book = CountdownBook::default();
        book.add(input.clone(), 0).unwrap();
        let due = book.items[0].target;
        assert_eq!(book.tick(due).len(), 1);
        let mut restored = CountdownBook::restore(book.items.clone()).unwrap();
        assert!(restored.tick(due + 60).is_empty());
        let mut past = CountdownBook::default();
        past.add(input, due + 60).unwrap();
        assert!(past.tick(due + 61).is_empty());
        let mut invalid = book.items.clone();
        invalid[0].target += 1;
        assert!(CountdownBook::restore(invalid).is_err());
    }
}
