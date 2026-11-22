use super::{field, issue};
use crate::{host::Host, jobs::Task};
use eframe::egui::{self, Ui};
use oxbox_core::tools::currency::{self, RateBook};
use serde::{Deserialize, Serialize};

fn fetched_time(timestamp: u64) -> String {
    i64::try_from(timestamp)
        .ok()
        .and_then(|timestamp| oxbox_core::tools::world_time::clock_at("UTC", timestamp).ok())
        .map_or_else(
            || "unknown time".into(),
            |clock| format!("{} {}", clock.date, clock.time),
        )
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    book: Option<RateBook>,
    network_enabled: bool,
}
pub(super) struct CurrencyTools {
    stored: Stored,
    loaded: bool,
    ready: bool,
    amount: String,
    from: String,
    to: String,
    job: Task<RateBook>,
    error: String,
    notice: String,
}
impl Default for CurrencyTools {
    fn default() -> Self {
        Self {
            stored: Stored::default(),
            loaded: false,
            ready: false,
            amount: "1".into(),
            from: "EUR".into(),
            to: "USD".into(),
            job: Task::default(),
            error: String::new(),
            notice: String::new(),
        }
    }
}
impl CurrencyTools {
    pub fn tick(&mut self, host: &mut Host) {
        if !self.loaded {
            self.loaded = true;
            let loaded = (|| {
                if let Some(stored) = host.load::<Stored>("native-currency.json")? {
                    return Ok(stored);
                }
                let legacy = host.load::<serde_json::Value>("currency.json")?;
                let book = legacy
                    .and_then(|value| value.get("currencyRates").cloned())
                    .map(serde_json::from_value::<RateBook>)
                    .transpose()
                    .map_err(|error| {
                        oxbox_core::CoreError::InvalidInput(format!(
                            "Currency cache could not be imported: {error}"
                        ))
                    })?;
                let stored = Stored {
                    book,
                    network_enabled: false,
                };
                host.save("native-currency.json", &stored)?;
                Ok::<_, oxbox_core::CoreError>(stored)
            })();
            match loaded {
                Ok(stored) => {
                    self.stored = stored;
                    self.ready = true;
                }
                Err(error) => self.error = error.to_string(),
            }
        }
        self.job.poll();
        if let Some(book) = self.job.result.take() {
            self.accept_book(host, book);
        }
    }
    fn accept_book(&mut self, host: &Host, book: RateBook) {
        let candidate = Stored {
            book: Some(book),
            network_enabled: self.stored.network_enabled,
        };
        match host.save("native-currency.json", &candidate) {
            Ok(()) => {
                self.stored = candidate;
                self.error.clear();
                self.notice = "Reference rates updated.".into();
            }
            Err(error) => {
                self.error = format!(
                    "Could not save new reference rates; the previous active table is unchanged: {error}"
                );
            }
        }
    }
    /// Manual imports/refreshes are ordinary view jobs. Leaving discards a late
    /// candidate; only an admitted successful result reaches the cache commit.
    pub(super) fn leave(&mut self) {
        self.job.abandon();
        self.notice.clear();
    }
    pub fn shutdown(&mut self) {
        self.job.shutdown();
    }
    pub fn show(&mut self, ui: &mut Ui, host: &mut Host) {
        self.tick(host);
        ui.label("Dated ECB reference rates. Conversion and XML import work offline. Downloads require explicit opt-in and a manual Refresh; no background refresh occurs.");
        if ui
            .button("Open approved currency network issue #1")
            .clicked()
            && let Err(error) = open::that("https://github.com/ninad-k/oxbox/issues/1")
        {
            self.error = format!("Could not open the review issue: {error}");
        }
        let previous = self.stored.network_enabled;
        ui.add_enabled_ui(self.ready && !self.job.running(), |ui| {
            ui.checkbox(
                &mut self.stored.network_enabled,
                "Allow manual ECB rate downloads",
            );
        });
        if previous != self.stored.network_enabled
            && let Err(error) = host.save("native-currency.json", &self.stored)
        {
            self.stored.network_enabled = previous;
            self.error = error.to_string();
        }
        ui.add_enabled_ui(self.ready && !self.job.running(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Import ECB XML…").clicked()
                    && let Some(path) = crate::host::dialogs::file(ui.ctx())
                        .add_filter("ECB XML", &["xml"])
                        .pick_file()
                {
                    self.notice.clear();
                    self.job.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        let book = currency::import(&path, super::time::unix_now() as u64)?;
                        cancel.check()?;
                        Ok(book)
                    });
                }
                if ui
                    .add_enabled(
                        self.stored.network_enabled,
                        egui::Button::new("Refresh from ECB"),
                    )
                    .clicked()
                {
                    self.notice.clear();
                    self.job.start(ui.ctx(), move |cancel, _| {
                        currency::download_with_cancel(true, &cancel)
                    });
                }
            });
        });
        self.job.show(ui);
        if !self.notice.is_empty() {
            crate::accessibility::status(ui, &self.notice);
        }
        if let Some(book) = &self.stored.book {
            let fetched = fetched_time(book.fetched_at);
            ui.label(format!(
                "Reference date: {} · fetched {} UTC · provider ECB",
                book.date, fetched
            ));
            if currency::needs_refresh(book.fetched_at, super::time::unix_now() as u64) {
                ui.label("This cache is more than a day old (or the clock changed). Import or manually refresh when appropriate.");
            }
            ui.label("Reference rates are not live transaction quotes.");
            field(ui, "Amount", &mut self.amount);
            for (from, label) in [(true, "From currency"), (false, "To currency")] {
                let selected = if from { &mut self.from } else { &mut self.to };
                let choice_response = egui::ComboBox::from_label(label)
                    .selected_text(selected.as_str())
                    .show_ui(ui, |ui| {
                        for currency in book.rates.keys() {
                            ui.selectable_value(selected, currency.clone(), currency);
                        }
                    });
                crate::accessibility::combo_expansion(&choice_response.response);
            }
            if ui.button("Swap currencies").clicked() {
                std::mem::swap(&mut self.from, &mut self.to);
            }
            if !self.amount.trim().is_empty() {
                match self
                    .amount
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Enter a finite amount.".to_string())
                    .and_then(|amount| {
                        currency::convert(amount, &self.from, &self.to, book)
                            .map_err(|error| error.to_string())
                    }) {
                    Ok(value) => {
                        ui.monospace(value.to_string());
                        if ui.button("Copy converted amount").clicked() {
                            ui.ctx().copy_text(value.to_string());
                        }
                    }
                    Err(error) => issue(ui, error),
                }
            }
        } else {
            ui.label("Import a dated rate table or opt in and Refresh before converting.");
        }
        if !self.error.is_empty() {
            issue(ui, &self.error);
        }
    }
}

#[cfg(test)]
mod functional_tests {
    use super::*;
    use std::{
        collections::BTreeMap,
        time::{Duration, Instant},
    };

    #[test]
    fn native_currency_adapter_preserves_utc_fetch_labels_and_unknown_bounds() {
        assert_eq!(fetched_time(0), "1970-01-01 00:00:00");
        assert_eq!(fetched_time(1), "1970-01-01 00:00:01");
        assert_eq!(fetched_time(253_402_300_800), "+10000-01-01 00:00:00");
        assert_eq!(fetched_time(u64::MAX), "unknown time");
        assert_eq!(fetched_time(i64::MAX as u64), "unknown time");
    }

    fn book(usd: f64) -> RateBook {
        RateBook {
            date: "2026-11-20".into(),
            fetched_at: 1,
            rates: BTreeMap::from([("EUR".into(), 1.0), ("USD".into(), usd)]),
        }
    }
    fn fixture() -> (tempfile::TempDir, Host, CurrencyTools) {
        let directory = tempfile::tempdir().unwrap();
        let host = Host::new(directory.path().to_path_buf()).unwrap();
        let tools = CurrencyTools {
            loaded: true,
            ready: true,
            stored: Stored {
                book: Some(book(1.1)),
                network_enabled: false,
            },
            ..Default::default()
        };
        host.save("native-currency.json", &tools.stored).unwrap();
        (directory, host, tools)
    }
    #[test]
    fn successful_cache_acceptance_commits_disk_and_announces_completion() {
        let (_directory, mut host, mut tools) = fixture();
        tools.job.result = Some(book(1.2));
        tools.tick(&mut host);
        let saved: Stored = host.load("native-currency.json").unwrap().unwrap();
        assert_eq!(
            saved.book.unwrap().rates["USD"],
            1.2,
            "completed rates never reached the persisted cache"
        );
        assert_eq!(tools.stored.book.as_ref().unwrap().rates["USD"], 1.2);
        let nodes = crate::accessibility::test_tree(|ui| tools.show(ui, &mut host));
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == egui::accesskit::Role::Status
                    && node
                        .label()
                        .is_some_and(|label| label.contains("Reference rates updated"))),
            "consuming the result lost its visible/live completion state"
        );
        tools.shutdown();
        host.shutdown();
    }
    #[test]
    fn failed_cache_commit_preserves_previous_in_memory_and_backed_up_book() {
        let (directory, mut host, mut tools) = fixture();
        let file = directory.path().join("native-currency.json");
        let backup = directory.path().join("prior-cache.json");
        std::fs::rename(&file, &backup).unwrap();
        std::fs::create_dir(&file).unwrap();
        tools.job.result = Some(book(1.2));
        tools.tick(&mut host);
        assert_eq!(
            tools.stored.book.as_ref().unwrap().rates["USD"],
            1.1,
            "unsaved result replaced the old active table"
        );
        let previous: Stored = serde_json::from_slice(&std::fs::read(backup).unwrap()).unwrap();
        assert_eq!(previous.book.unwrap().rates["USD"], 1.1);
        assert!(!tools.error.is_empty(), "cache save failure was hidden");
        tools.shutdown();
        host.shutdown();
    }
    #[test]
    fn abandoned_fetch_cannot_publish_a_late_cache_or_result() {
        let (directory, mut host, mut tools) = fixture();
        let before = std::fs::read(directory.path().join("native-currency.json")).unwrap();
        let (ready_send, ready_receive) = std::sync::mpsc::channel();
        let (release_send, release_receive) = std::sync::mpsc::channel();
        assert!(tools.job.start(&egui::Context::default(), move |_, _| {
            ready_send.send(()).unwrap();
            release_receive
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            Ok(book(1.2))
        }));
        ready_receive.recv_timeout(Duration::from_secs(2)).unwrap();
        tools.job.abandon();
        release_send.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while tools.job.running() && Instant::now() < deadline {
            tools.tick(&mut host);
            std::thread::yield_now();
        }
        assert!(!tools.job.running());
        assert!(tools.job.result.is_none());
        assert_eq!(tools.stored.book.as_ref().unwrap().rates["USD"], 1.1);
        assert_eq!(
            std::fs::read(directory.path().join("native-currency.json")).unwrap(),
            before
        );
        tools.shutdown();
        host.shutdown();
    }
}
