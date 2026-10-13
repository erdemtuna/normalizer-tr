//! Check common workload registration against metadata without clocks.
#![cfg(feature = "serde")]
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

#[test]
fn common_cases_match_independent_inventory_without_shared_omissions() {
    let inventory: serde_json::Value =
        serde_json::from_str(include_str!("../benches/latency/workload_inventory.json")).unwrap();
    let required: std::collections::BTreeSet<_> = support::inventory_keys(&inventory)
        .into_iter()
        .filter(|key| {
            matches!(
                key.0.as_str(),
                "scanner" | "source-map" | "unicode-fallback" | "resources"
            )
        })
        .collect();
    let mut actual = std::collections::BTreeSet::new();
    for (suite, cases) in [
        ("scanner", scanner_scaling::cases()),
        ("source-map", source_mapping::cases()),
        ("unicode-fallback", unicode_fallback_scaling::cases()),
        ("resources", support::resource_cases()),
    ] {
        for case in cases {
            assert!(actual.insert((
                suite.to_owned(),
                case.id.to_owned(),
                case.text.len(),
                support::policy_name(case.options.ambiguity_policy).to_owned(),
            )));
        }
    }
    assert_eq!(actual.len(), 309);
    assert_eq!(actual, required);
}
