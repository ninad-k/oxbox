use super::{field, issue};
use crate::{host::Host, jobs::Task};
use eframe::egui::{self, Ui};
use oxbox_core::tools::currency::{self, RateBook};
use serde::{Deserialize, Serialize};
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
            self.stored.book = Some(book);
            self.error.clear();
        }
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
                    let directory = host.data_dir().to_path_buf();
                    let network_enabled = self.stored.network_enabled;
                    self.job.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        let book = currency::import(&path, super::time::unix_now() as u64)?;
                        cancel.check()?;
                        crate::host::save_at(
                            &directory,
                            "native-currency.json",
                            &Stored {
                                book: Some(book.clone()),
                                network_enabled,
                            },
                        )?;
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
                    let directory = host.data_dir().to_path_buf();
                    let network_enabled = self.stored.network_enabled;
                    self.job.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        let book = currency::download(true)?;
                        cancel.check()?;
                        crate::host::save_at(
                            &directory,
                            "native-currency.json",
                            &Stored {
                                book: Some(book.clone()),
                                network_enabled,
                            },
                        )?;
                        Ok(book)
                    });
                }
            });
        });
        self.job.show(ui);
        if let Some(book) = &self.stored.book {
            let fetched = i64::try_from(book.fetched_at)
                .ok()
                .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
                .map_or_else(
                    || "unknown time".into(),
                    |value| value.format("%Y-%m-%d %H:%M:%S").to_string(),
                );
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
                egui::ComboBox::from_label(label)
                    .selected_text(selected.as_str())
                    .show_ui(ui, |ui| {
                        for currency in book.rates.keys() {
                            ui.selectable_value(selected, currency.clone(), currency);
                        }
                    });
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
