//! Shared, compiled policy fixtures with explicit deterministic catalog order.
use normalizer_tr::{Hint, HintKind, SourceRange};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn hint(case: &Value) -> Option<Hint> {
    let hint = case.get("hint")?;
    let kind = match hint["kind"].as_str().unwrap() {
        "cardinal" => HintKind::Cardinal,
        "digits" => HintKind::Digits,
        "date" => HintKind::Date,
        "time" => HintKind::Time,
        "ordinal" => HintKind::Ordinal,
        "roman" => HintKind::Roman,
        "range" => HintKind::Range,
        "telephone" => HintKind::Telephone,
        "electronic" => HintKind::Electronic,
        other => panic!("unsupported fixture hint {other}"),
    };
    Some(Hint::new(
        SourceRange::new(
            hint["start"].as_u64().unwrap() as usize,
            hint["end"].as_u64().unwrap() as usize,
        ),
        kind,
    ))
}

pub fn composable(case: &Value) -> bool {
    case.get("composable").is_none_or(|value| {
        value
            .as_bool()
            .expect("policy fixture composable metadata must be boolean")
    })
}

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
    (
        "policy-contract/pronunciation-nominal.json",
        include_str!("../fixtures/policy-contract/pronunciation-nominal.json"),
    ),
    (
        "policy-contract/numeric-tails.json",
        include_str!("../fixtures/policy-contract/numeric-tails.json"),
    ),
    (
        "policy-contract/temporal-notation.json",
        include_str!("../fixtures/policy-contract/temporal-notation.json"),
    ),
    (
        "policy-contract/composite-notation.json",
        include_str!("../fixtures/policy-contract/composite-notation.json"),
    ),
    (
        "policy-contract/review-notation-admission.json",
        include_str!("../fixtures/policy-contract/review-notation-admission.json"),
    ),
    (
        "policy-contract/standalone-letters.json",
        include_str!("../fixtures/policy-contract/standalone-letters.json"),
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
        let _ = hint(case);
        let _ = composable(case);
        assert!(
            ids.insert(case["id"].as_str().unwrap()),
            "duplicate policy-contract case"
        );
    }
    (cases, bytes)
}
