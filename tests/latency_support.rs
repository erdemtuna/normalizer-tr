//! Evidence serialization tests without invoking the latency runner.
#![cfg(feature = "serde")]
#[allow(dead_code)]
#[path = "../benches/latency/support.rs"]
mod support;

use normalizer_tr::{
    Hint, HintKind, LimitKind, NormalizeError, NormalizeOptions, Normalizer, SourceRange,
};
use serde_json::json;

#[test]
fn snapshots_preserve_exact_options_and_structured_failures() {
    let options = NormalizeOptions {
        hints: vec![Hint::new(SourceRange::new(1, 3), HintKind::Cardinal)],
        ..Default::default()
    };
    let fingerprint = support::input_fingerprint("ö1", &options);
    assert_eq!(fingerprint["text"], "ö1");
    assert_eq!(fingerprint["options"]["ambiguity_policy"], "Preserve");
    assert_eq!(
        fingerprint["options"]["hints"],
        json!([{"range":{"start":1,"end":3},"kind":"Cardinal"}])
    );
    assert_eq!(
        support::outcome(Err(NormalizeError::LimitExceeded(LimitKind::Result))),
        json!({"error":"LimitExceeded","limit":"Result"})
    );
    assert_eq!(
        support::outcome(Err(NormalizeError::Cancelled)),
        json!({"error":"Cancelled"})
    );
}

#[test]
fn rich_rows_and_quantiles_do_not_need_timed_calls() {
    let mut values: Vec<_> = (1..=100).rev().collect();
    let timing = support::quantiles(&mut values);
    assert_eq!(
        timing,
        json!({"count":100,"p50_ns":50,"p95_ns":95,"p99_ns":99,"max_ns":100})
    );
    let row = support::scaling_row(
        "test",
        "invalid-input",
        "",
        &NormalizeOptions::default(),
        timing,
        support::outcome(Err(NormalizeError::InvalidInput)),
    );
    assert_eq!(row["bytes"], 0);
    assert_eq!(row["policy"], "preserve");
    assert_eq!(row["outcome"], json!({"error":"InvalidInput"}));
    assert_eq!(support::SCALING_SAMPLES, 100);
}

#[test]
fn explicit_input_and_hint_limits_are_checked_without_sampling() {
    let normalizer = Normalizer::new().unwrap();
    let cases = support::resource_cases();
    assert_eq!(cases.len(), 6);
    for case in cases {
        support::check_resource(&case, &normalizer.normalize(&case.text, &case.options));
    }
    let inventory: serde_json::Value =
        serde_json::from_str(include_str!("../benches/latency/workload_inventory.json")).unwrap();
    assert_eq!(support::inventory_keys(&inventory).len(), 625);
}
