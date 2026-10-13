//! Validate source-map measurement inventory and semantics without timed calls.
#![cfg(feature = "serde")]
#[path = "../benches/latency/source_mapping.rs"]
mod source_mapping;
#[allow(dead_code)]
#[path = "../benches/latency/support.rs"]
mod support;

use std::collections::BTreeSet;

use normalizer_tr::{LimitKind, NormalizeError, Normalizer};

#[test]
fn inventory_has_exact_main_hint_and_error_controls() {
    let cases = source_mapping::cases();
    let mut actual = BTreeSet::new();
    for case in &cases {
        assert!(actual.insert((
            case.id,
            case.text.len(),
            support::policy_name(case.options.ambiguity_policy),
        )));
    }
    let mut expected = BTreeSet::new();
    for id in [
        "ascii-single-word",
        "ascii-many-words",
        "ascii-crlf-sparse",
        "ascii-crlf-dense",
        "unicode-nfc",
        "unicode-decomposed",
        "unicode-nfc-late",
    ] {
        for bytes in [256, 1024, 4096, 16384, 32768] {
            for policy in ["preserve", "reject", "fallback"] {
                expected.insert((id, bytes, policy));
            }
        }
    }
    for id in [
        "hints-zero",
        "hints-one",
        "hints-255",
        "hints-256",
        "hints-reversed-valid",
        "hints-invalid-early",
        "hints-invalid-bounds",
        "hints-invalid-owner",
        "policy-unresolved",
    ] {
        for policy in ["preserve", "reject", "fallback"] {
            expected.insert((id, 1024, policy));
        }
    }
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 132);
    let hinted = cases.iter().filter(|case| {
        matches!(
            case.id,
            "hints-zero" | "hints-one" | "hints-255" | "hints-256" | "hints-reversed-valid"
        )
    });
    let expected_text = "1;\r\n".repeat(256);
    for case in hinted {
        assert_eq!(case.text, expected_text);
        let count = match case.id {
            "hints-zero" => 0,
            "hints-one" => 1,
            "hints-255" => 255,
            _ => 256,
        };
        assert_eq!(case.options.hints.len(), count);
        if case.id == "hints-reversed-valid" {
            assert!(
                case.options
                    .hints
                    .windows(2)
                    .all(|pair| pair[0].range().start() > pair[1].range().start())
            );
        }
    }
}

#[test]
fn every_workload_has_independently_checked_repeatable_public_outcomes() {
    let normalizer = Normalizer::new().unwrap();
    for case in source_mapping::cases() {
        let before = normalizer.normalize(&case.text, &case.options);
        source_mapping::check(&case, &before);
        let after = normalizer.normalize(&case.text, &case.options);
        source_mapping::check(&case, &after);
        assert_eq!(after, before, "{}", case.id);
        let fingerprint = support::input_fingerprint(&case.text, &case.options);
        assert_eq!(fingerprint["text"], case.text);
        assert_eq!(
            fingerprint["options"]["hints"].as_array().unwrap().len(),
            case.options.hints.len()
        );
        let outcome = support::outcome(before);
        if case.id.starts_with("hints-invalid-") {
            assert_eq!(outcome, serde_json::json!({"error": "InvalidHint"}));
        }
    }
}

#[test]
fn independent_validator_rejects_success_shaped_and_wrong_error_outcomes() {
    let cases = source_mapping::cases();
    let invalid = cases
        .iter()
        .find(|case| case.id == "hints-invalid-bounds")
        .unwrap();
    assert!(
        std::panic::catch_unwind(|| {
            source_mapping::check(
                invalid,
                &Err(NormalizeError::LimitExceeded(LimitKind::Input)),
            );
        })
        .is_err()
    );
    let plain = cases
        .iter()
        .find(|case| case.id == "ascii-single-word")
        .unwrap();
    let different = Normalizer::new().unwrap().normalize("1", &plain.options);
    assert!(std::panic::catch_unwind(|| source_mapping::check(plain, &different)).is_err());
}
