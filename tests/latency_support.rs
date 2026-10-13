//! Evidence serialization tests without invoking the latency runner.
#![cfg(feature = "serde")]
#[allow(dead_code)]
#[path = "../benches/latency/support.rs"]
mod support;

use normalizer_tr::{
    Hint, HintKind, LimitKind, NormalizeError, NormalizeOptions, Normalizer, SourceRange,
};
use serde_json::json;
use std::io::{self, Write};

struct FailingWriter {
    fail_on_flush: bool,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_on_flush {
            Ok(bytes.len())
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "write failed"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.fail_on_flush {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "flush failed",
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn buffered_report_preserves_pretty_json_bytes() {
    let report = json!({
        "input": "x".repeat(32 * 1024),
        "unicode": "ö\n\"",
        "outcome": {"error": "LimitExceeded", "limit": "Result"},
        "timing": {"p50_ns": 1, "p95_ns": 2},
    });
    let mut bytes = Vec::new();
    support::write_report(&mut bytes, &report).unwrap();
    assert_eq!(bytes, serde_json::to_vec_pretty(&report).unwrap());
}

#[test]
fn buffered_report_propagates_write_and_flush_errors() {
    let report = json!({"input": "x".repeat(32 * 1024)});
    for (fail_on_flush, expected_kind) in [
        (false, io::ErrorKind::BrokenPipe),
        (true, io::ErrorKind::PermissionDenied),
    ] {
        let error = support::write_report(FailingWriter { fail_on_flush }, &report).unwrap_err();
        assert_eq!(error.kind(), expected_kind);
        assert_eq!(
            error.to_string(),
            if fail_on_flush {
                "flush failed"
            } else {
                "write failed"
            }
        );
    }
}

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
