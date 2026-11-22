//! Deterministic public-API observations. Capture with Chrono before migrating;
//! compare the same generated inputs with the committed historical fixture later.
use oxbox_core::{
    CoreError, CoreResult,
    tools::{
        alarm::{self, Alarm, AlarmBook, AlarmInput, Ambiguity, Recurrence},
        currency, world_time,
    },
};
use serde::Serialize;
use serde_json::{Value, json};

fn outcome<T: Serialize>(result: CoreResult<T>) -> CoreResult<Value> {
    match result {
        Ok(value) => serde_json::to_value(value)
            .map(|value| json!({"ok": value}))
            .map_err(|error| CoreError::InvalidInput(error.to_string())),
        Err(error) => Ok(json!({"error": {"code": error.code(), "message": error.to_string()}})),
    }
}
fn clock_outcome(zone: &str, timestamp: i64) -> CoreResult<Value> {
    match std::panic::catch_unwind(|| world_time::clock_at(zone, timestamp)) {
        Ok(result) => outcome(result),
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_owned())
                })
                .unwrap_or_else(|| "non-string panic".into());
            Ok(json!({"panic":message}))
        }
    }
}
fn currency_dates() -> CoreResult<Vec<Value>> {
    [
        "1900-02-28",
        "1900-02-29",
        "2000-02-29",
        "2024-02-29",
        "2023-02-29",
        "2024-04-31",
        "2024-2-9",
        "2024-02-9",
        "2024-2-09",
        " 2024-02-29",
        "2024-02-29 ",
        "0000-01-01",
        "0001-01-01",
        "9999-12-31",
        "+2024-02-29",
        "-0001-01-01",
        "",
    ]
    .into_iter()
    .map(|date| {
        let xml =
            format!("<Cube><Cube time='{date}'><Cube currency='USD' rate='1.25'/></Cube></Cube>");
        let result = currency::parse_ecb(&xml, 42).and_then(|book| {
            let stored = serde_json::to_value(&book)
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
            let restored: currency::RateBook = serde_json::from_value(stored.clone())
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
            Ok(json!({"date":book.date,"stored":stored,"roundtrip":restored}))
        });
        Ok(json!({"dateInput":date,"result":outcome(result)?}))
    })
    .collect()
}
fn input(local: &str, zone: &str, recurrence: Recurrence, ambiguity: Ambiguity) -> AlarmInput {
    AlarmInput {
        label: "Generated café alarm".into(),
        local_date_time: local.into(),
        zone: zone.into(),
        recurrence,
        ambiguity,
    }
}
fn utc(value: &str) -> CoreResult<i64> {
    alarm::resolve_local(value, "UTC", Ambiguity::Earlier)
}
fn saved_records() -> CoreResult<Value> {
    let mut book = AlarmBook::default();
    let once = book.add(
        input(
            "2030-01-01T00:00",
            "UTC",
            Recurrence::Once,
            Ambiguity::Earlier,
        ),
        0,
    )?;
    book.add(
        input(
            "2024-11-03T01:30:45",
            "US/Eastern",
            Recurrence::Daily,
            Ambiguity::Later,
        ),
        0,
    )?;
    let weekday = book.add(
        input(
            "2030-01-04T10:15",
            "Pacific/Chatham",
            Recurrence::Weekdays,
            Ambiguity::Earlier,
        ),
        0,
    )?;
    book.set_enabled(weekday, false, 0)?;
    let initial = serde_json::to_value(&book.items)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let due = utc("2030-01-01T00:00")?;
    let fired = book.tick(due);
    let encoded = serde_json::to_string(&book.items)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let items = serde_json::from_str(&encoded)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let mut restored = AlarmBook::restore(items)?;
    let roundtrip = serde_json::to_value(&restored.items)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let repeated = restored.tick(due + 1);
    restored.snooze(once, due, 5)?;
    let before_snooze = restored.tick(due + 299);
    let at_snooze = restored.tick(due + 300);
    restored.dismiss(once)?;
    Ok(
        json!({"initial":initial,"fired":fired,"roundtrip":roundtrip,"repeated":repeated,"beforeSnooze":before_snooze,"atSnooze":at_snooze,"afterDismiss":restored.items}),
    )
}
fn restore_cases() -> CoreResult<Vec<Value>> {
    let mut book = AlarmBook::default();
    book.add(
        input(
            "2030-01-01T00:00",
            "UTC",
            Recurrence::Once,
            Ambiguity::Later,
        ),
        0,
    )?;
    let original = book.items[0].clone();
    let mut cases: Vec<(&str, Vec<Alarm>)> = vec![
        ("valid", vec![original.clone()]),
        ("duplicate-id", vec![original.clone(), original.clone()]),
    ];
    for (name, mutate) in [
        ("zero-id", 0_u8),
        ("too-large-id", 1),
        ("invalid-next-at", 2),
        ("wide-chrono-next-at", 3),
        ("gap-local-record", 4),
        ("unbounded-last-fired", 5),
    ] {
        let mut record = original.clone();
        match mutate {
            0 => record.id = 0,
            1 => record.id = 9_007_199_254_740_992,
            2 => record.next_at = i64::MAX,
            3 => record.next_at = 8_210_266_876_799,
            4 => {
                record.input.local_date_time = "2024-03-10T02:30".into();
                record.input.zone = "America/New_York".into();
            }
            _ => record.last_fired = Some(i64::MAX),
        }
        cases.push((name, vec![record]));
    }
    cases
        .into_iter()
        .map(|(name, records)| {
            let result = outcome(AlarmBook::restore(records.clone()).map(|book| book.items))?;
            Ok(json!({"name":name,"records":records,"result":result}))
        })
        .collect()
}
pub fn snapshot() -> CoreResult<Value> {
    let zones = world_time::zones();
    let mut clocks = Vec::new();
    for zone in &zones {
        for timestamp in [0, 1_704_067_200, 1_720_000_000] {
            clocks.push(
                json!({"zone":zone,"timestamp":timestamp,"result":clock_outcome(zone,timestamp)?}),
            );
        }
    }
    for zone in [
        "UTC",
        "US/Eastern",
        "America/New_York",
        "Europe/London",
        "Europe/Kiev",
        "Asia/Calcutta",
        "Asia/Kathmandu",
        "Australia/Lord_Howe",
        "Pacific/Chatham",
        "Pacific/Apia",
        "Etc/GMT+5",
        "Zulu",
    ] {
        for timestamp in [
            -2_208_988_800,
            -1,
            1_710_053_999,
            1_710_054_000,
            1_730_613_599,
            1_730_613_600,
        ] {
            clocks.push(
                json!({"zone":zone,"timestamp":timestamp,"result":clock_outcome(zone,timestamp)?}),
            );
        }
    }
    // Original observed Chrono UTC limits, frozen in chrono-time-compat.json.
    // Keep reference inputs independent of either runtime time library.
    let minimum = -8_334_601_228_800_i64;
    let maximum = 8_210_266_876_799_i64;
    let mut clock_ranges = Vec::new();
    for zone in [
        "UTC",
        "America/New_York",
        "Pacific/Kiritimati",
        "Etc/GMT+12",
    ] {
        for timestamp in [
            i64::MIN,
            minimum - 1,
            minimum,
            -62_167_219_200,
            -62_135_596_800,
            253_402_300_799,
            253_402_300_800,
            maximum,
            maximum + 1,
            i64::MAX,
        ] {
            clock_ranges.push(
                json!({"zone":zone,"timestamp":timestamp,"result":clock_outcome(zone,timestamp)?}),
            );
        }
    }
    let searches = [
        "",
        "UTC",
        " new_york ",
        "NEW YORK",
        "Asia/Calcutta",
        "Etc/GMT+5",
        "not a real time zone",
    ]
    .map(|query| json!({"query":query,"result":world_time::search_zones(query)}));
    let long_zone = "Z".repeat(129);
    let mut local_times = Vec::new();
    for (local, zone) in [
        ("2024-03-10T02:30", "America/New_York"),
        ("2024-11-03T01:30", "America/New_York"),
        ("2024-11-03T01:30:45", "US/Eastern"),
        ("2024-03-31T01:30", "Europe/London"),
        ("2024-10-27T01:30", "Europe/London"),
        ("2024-04-07T01:45", "Australia/Lord_Howe"),
        ("2011-12-30T12:00", "Pacific/Apia"),
        ("2030-01-01T10:00", "Asia/Kathmandu"),
        ("2024-02-29T23:59:59", "Asia/Calcutta"),
        ("0001-01-01T00:00", "UTC"),
        ("9999-12-31T23:59:59", "UTC"),
        ("2024-1-2T3:4", "UTC"),
        ("2024-01-02T03:04:05", "UTC"),
        ("2016-12-31T23:59:60", "UTC"),
        ("+2024-01-02T03:04", "UTC"),
        ("0000-01-01T00:00", "UTC"),
        ("10000-01-01T00:00", "UTC"),
        ("2023-02-29T10:00", "UTC"),
        ("bad", "UTC"),
        ("2030-01-01T10:00", "Not/AZone"),
        ("2030-01-01T10:00", "utc"),
        ("2030-01-01T10:00", long_zone.as_str()),
    ] {
        for ambiguity in [Ambiguity::Earlier, Ambiguity::Later] {
            local_times.push(json!({"local":local,"zone":zone,"ambiguity":ambiguity,"result":outcome(alarm::resolve_local(local,zone,ambiguity))?}));
        }
    }
    let mut recurrences = Vec::new();
    for (local, zone, after) in [
        ("2030-01-04T10:00", "UTC", "2030-01-04T10:00"),
        ("2030-01-04T10:00", "UTC", "2030-01-05T12:00"),
        ("2024-03-09T02:30", "America/New_York", "2024-03-09T08:00"),
        ("2024-11-03T01:30", "America/New_York", "2024-11-03T05:45"),
        ("2024-11-03T01:30", "America/New_York", "2024-11-03T06:45"),
        (
            "2024-04-07T01:45",
            "Australia/Lord_Howe",
            "2024-04-06T15:00",
        ),
        ("2011-12-29T12:00", "Pacific/Apia", "2011-12-29T23:00"),
        ("9999-12-31T23:59:59", "UTC", "9999-12-31T23:59:59"),
    ] {
        let after_timestamp = utc(after)?;
        for recurrence in [Recurrence::Once, Recurrence::Daily, Recurrence::Weekdays] {
            for ambiguity in [Ambiguity::Earlier, Ambiguity::Later] {
                let input = input(local, zone, recurrence, ambiguity);
                recurrences.push(json!({"input":input,"after":after_timestamp,"result":outcome(alarm::next_occurrence(&input,after_timestamp))?}));
            }
        }
    }
    Ok(
        json!({"schema":1,"contract":"oxbox-time-compat-v1","zones":zones,"clocks":clocks,"clockRanges":clock_ranges,"searches":searches,"localTimes":local_times,"recurrences":recurrences,"savedRecords":saved_records()?,"restoreCases":restore_cases()?,"currencyDates":currency_dates()?}),
    )
}
