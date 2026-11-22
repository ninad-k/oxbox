//! Frozen pre-migration behavior, captured by the integrator with export_time_compat.
#[path = "support/time_compat.rs"]
mod compatibility;

fn expected() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/chrono-time-compat.json");
    let bytes = std::fs::read(path).expect(
        "capture the original corpus with --example export_time_compat before migrating time APIs",
    );
    assert!(
        bytes.len() <= 4 * 1024 * 1024,
        "the generated fixture must stay bounded"
    );
    serde_json::from_slice(&bytes).expect("the captured compatibility fixture must be valid JSON")
}
fn assert_sections(sections: &[&str]) {
    let expected = expected();
    let actual =
        compatibility::snapshot().expect("deterministic current public APIs must be observable");
    assert_eq!(actual["schema"], expected["schema"]);
    for section in sections {
        if let (Some(actual), Some(expected)) =
            (actual[section].as_array(), expected[section].as_array())
        {
            assert_eq!(actual.len(), expected.len(), "{section} entry count");
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                if expected
                    .get("result")
                    .and_then(|result| result.get("panic"))
                    .is_some()
                {
                    // A recorded legacy panic is a defect, not a behavior to
                    // preserve. The explicit no-panic test below requires a
                    // checked invalid-input error after remediation.
                    continue;
                }
                assert_eq!(actual, expected, "{section} first differing row {index}");
            }
        } else {
            assert_eq!(actual[section], expected[section], "{section}");
        }
    }
}
#[test]
fn time_compat_preserves_every_zone_alias_search_and_clock_json_field() {
    assert_sections(&["zones", "searches", "clocks", "clockRanges"]);
}
#[test]
fn time_compat_preserves_local_gap_fold_grammar_and_recurrence_results() {
    assert_sections(&["localTimes", "recurrences"]);
}
#[test]
fn time_compat_preserves_saved_alarm_records_and_restore_validation() {
    assert_sections(&["savedRecords", "restoreCases"]);
}
#[test]
fn time_compat_preserves_currency_date_acceptance_and_stored_strings() {
    assert_sections(&["currencyDates"]);
}
#[test]
fn time_compat_world_clock_edges_return_errors_instead_of_panicking() {
    let expected = expected();
    let actual = compatibility::snapshot().expect("clock edges must be observable");
    for (index, row) in actual["clockRanges"].as_array().unwrap().iter().enumerate() {
        assert!(
            row["result"].get("panic").is_none(),
            "world-clock edge {index} must never panic: {row}"
        );
        if expected["clockRanges"][index]["result"]
            .get("panic")
            .is_some()
        {
            assert_eq!(
                row["result"]["error"]["code"], "invalid_input",
                "unrepresentable local edge requires a checked error"
            );
        }
    }
}
