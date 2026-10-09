//! Shared, compiled policy fixtures with explicit deterministic catalog order.
use serde_json::Value;
use std::collections::BTreeSet;

const CATALOG: &str = include_str!("../fixtures/policy-contract.json");
const GROUPS: &[(&str, &str)] = &[
    (
        "policy-contract/money-grouping.json",
        include_str!("../fixtures/policy-contract/money-grouping.json"),
    ),
    (
        "policy-contract/quantity-notation.json",
        include_str!("../fixtures/policy-contract/quantity-notation.json"),
    ),
    (
        "policy-contract/lexical-readings.json",
        include_str!("../fixtures/policy-contract/lexical-readings.json"),
    ),
    (
        "policy-contract/source-punctuation.json",
        include_str!("../fixtures/policy-contract/source-punctuation.json"),
    ),
    (
        "policy-contract/date-intent.json",
        include_str!("../fixtures/policy-contract/date-intent.json"),
    ),
    (
        "policy-contract/unresolved-outcomes.json",
        include_str!("../fixtures/policy-contract/unresolved-outcomes.json"),
    ),
    (
        "policy-contract/source-context.json",
        include_str!("../fixtures/policy-contract/source-context.json"),
    ),
    (
        "policy-contract/compound-boundaries.json",
        include_str!("../fixtures/policy-contract/compound-boundaries.json"),
    ),
    (
        "policy-contract/initialisms.json",
        include_str!("../fixtures/policy-contract/initialisms.json"),
    ),
    (
        "policy-contract/pronunciation-variants.json",
        include_str!("../fixtures/policy-contract/pronunciation-variants.json"),
    ),
    (
        "policy-contract/initialism-boundaries.json",
        include_str!("../fixtures/policy-contract/initialism-boundaries.json"),
    ),
    (
        "policy-contract/pronunciation-ai.json",
        include_str!("../fixtures/policy-contract/pronunciation-ai.json"),
    ),
    (
        "policy-contract/pronunciation-developer.json",
        include_str!("../fixtures/policy-contract/pronunciation-developer.json"),
    ),
    (
        "policy-contract/pronunciation-consumer.json",
        include_str!("../fixtures/policy-contract/pronunciation-consumer.json"),
    ),
    (
        "policy-contract/pronunciation-boundaries.json",
        include_str!("../fixtures/policy-contract/pronunciation-boundaries.json"),
    ),
    (
        "policy-contract/review-money-ownership.json",
        include_str!("../fixtures/policy-contract/review-money-ownership.json"),
    ),
    (
        "policy-contract/review-mixed-members.json",
        include_str!("../fixtures/policy-contract/review-mixed-members.json"),
    ),
    (
        "policy-contract/review-tight-admission.json",
        include_str!("../fixtures/policy-contract/review-tight-admission.json"),
    ),
    (
        "policy-contract/pronunciation-ema-lightning.json",
        include_str!("../fixtures/policy-contract/pronunciation-ema-lightning.json"),
    ),
];

pub fn load() -> (Vec<Value>, usize) {
    let catalog: Vec<String> = serde_json::from_str(CATALOG).unwrap();
    assert_eq!(
        catalog,
        GROUPS
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>()
    );
    let mut cases = Vec::new();
    let mut bytes = CATALOG.len();
    for (_, source) in GROUPS {
        let group: Vec<Value> = serde_json::from_str(source).unwrap();
        assert!(!group.is_empty());
        cases.extend(group);
        bytes += source.len();
    }
    let mut ids = BTreeSet::new();
    for case in &cases {
        assert!(
            ids.insert(case["id"].as_str().unwrap()),
            "duplicate policy-contract case"
        );
    }
    (cases, bytes)
}
