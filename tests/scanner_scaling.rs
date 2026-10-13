//! Untimed baseline checks for the common scanner performance inputs.
#![cfg(feature = "serde")]
#[path = "../benches/latency/scanner_scaling.rs"]
mod scanner_scaling;
#[allow(dead_code)]
#[path = "../benches/latency/support.rs"]
mod support;

use std::collections::BTreeSet;

use normalizer_tr::{AmbiguityPolicy, Normalizer};

#[test]
fn scanner_workload_inventory_is_exact_and_unique() {
    let cases = scanner_scaling::cases();
    assert_eq!(cases.len(), 123);
    let expected_families = [
        "list-no-comma-prose",
        "list-no-comma-single-identifier",
        "list-no-comma-many-identifiers",
        "list-approved-multiword",
        "list-late-unknown",
        "list-comma-prose",
        "list-candidate-limit",
        "address-period-prose",
        "address-url-negative",
        "address-decimal-negative",
        "address-explicit",
        "address-inferred",
        "address-postal-unpunctuated",
        "address-malformed-field",
        "address-candidate-limit",
    ];
    let mut expected = BTreeSet::new();
    for family in expected_families {
        let sizes: &[usize] = if family.ends_with("candidate-limit") {
            &[32768]
        } else {
            &[4096, 16384, 32768]
        };
        for &bytes in sizes {
            for policy in ["Preserve", "Reject", "Fallback"] {
                expected.insert((family, bytes, policy.to_owned()));
            }
        }
    }
    let actual: BTreeSet<_> = cases
        .iter()
        .map(|case| {
            assert!(case.options.hints.is_empty());
            (
                case.id,
                case.text.len(),
                format!("{:?}", case.options.ambiguity_policy),
            )
        })
        .collect();
    assert_eq!(actual.len(), cases.len());
    assert_eq!(actual, expected);
}

#[test]
fn unchanged_core_satisfies_every_scanner_oracle() {
    let normalizer = Normalizer::new().unwrap();
    for case in scanner_scaling::cases() {
        assert!(matches!(
            case.options.ambiguity_policy,
            AmbiguityPolicy::Preserve | AmbiguityPolicy::Reject | AmbiguityPolicy::Fallback
        ));
        scanner_scaling::check(&case, &normalizer.normalize(&case.text, &case.options));
    }
}
