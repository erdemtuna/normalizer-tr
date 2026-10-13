//! Check every prepared workload against metadata before any clocks.
#![cfg(feature = "serde")]
#[allow(dead_code)]
#[path = "../benches/latency/legacy_inputs.rs"]
mod legacy_inputs;
#[allow(dead_code)]
#[path = "../benches/latency/recognition_scaling.rs"]
mod recognition_scaling;
#[allow(dead_code)]
#[path = "../benches/latency/scanner_scaling.rs"]
mod scanner_scaling;
#[allow(dead_code)]
#[path = "../benches/latency/source_mapping.rs"]
mod source_mapping;
#[allow(dead_code)]
#[path = "../benches/latency/support.rs"]
mod support;
#[allow(dead_code)]
#[path = "../benches/latency/unicode_fallback_scaling.rs"]
mod unicode_fallback_scaling;

// The recognition module keeps its original parent helper calls.
use support::{outcome, quantiles};

fn inventory() -> serde_json::Value {
    serde_json::from_str(include_str!("../benches/latency/workload_inventory.json")).unwrap()
}

fn prepared() -> Vec<(&'static str, Vec<support::ScalingCase>)> {
    vec![
        ("expanded", legacy_inputs::expanded()),
        ("large", legacy_inputs::large()),
        ("limits", legacy_inputs::limits()),
        ("recognition", recognition_scaling::cases()),
        ("scanner", scanner_scaling::cases()),
        ("source-map", source_mapping::cases()),
        ("unicode-fallback", unicode_fallback_scaling::cases()),
        ("resources", support::resource_cases()),
    ]
}

#[test]
fn all_625_prepared_cases_match_independent_inventory_without_clocks() {
    let prepared = prepared();
    let suites: Vec<_> = prepared
        .iter()
        .map(|(suite, cases)| (*suite, cases.as_slice()))
        .collect();
    support::validate_prepared(&inventory(), &suites);
    assert_eq!(
        prepared
            .iter()
            .take(4)
            .map(|(_, cases)| cases.len())
            .sum::<usize>(),
        316
    );
    assert_eq!(support::inventory_keys(&inventory()).len(), 625);
}

#[test]
fn existing_limit_control_recipes_and_order_are_unchanged() {
    let cases = legacy_inputs::limits();
    let expected = [
        ("maximum-hints", "1;".repeat(256)),
        ("candidate-limit", "1 ".repeat(4097)),
        (
            "long-identifier",
            format!("https://ornek.com/{}", "a".repeat(16000)),
        ),
        (
            "invalid-compounds",
            "TR33 0006 1005 1978 6457 8413 2A; ".repeat(400),
        ),
    ];
    for (case, (id, text)) in cases.iter().zip(expected) {
        assert_eq!(case.id, id);
        assert_eq!(case.text, text);
    }
    assert_eq!(cases[0].options.hints.len(), 256);
    assert_eq!(cases[2].text.len(), 16018);
}

#[test]
#[should_panic(expected = "prepared workload coverage differs before timing")]
fn wrong_declared_byte_count_fails_before_any_normalizer_or_clock() {
    let mut inventory = inventory();
    for group in inventory["native"].as_array_mut().unwrap() {
        if group["suite"] == "limits" && group["ids"][0] == "long-identifier" {
            group["bytes"][0] = serde_json::json!(16017);
        }
    }
    let prepared = prepared();
    let suites: Vec<_> = prepared
        .iter()
        .map(|(suite, cases)| (*suite, cases.as_slice()))
        .collect();
    support::validate_prepared(&inventory, &suites);
}
