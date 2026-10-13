//! Independent Unicode catalog checks without invoking sampler timing.
#![cfg(feature = "serde")]

#[allow(dead_code)]
#[path = "../benches/latency/support.rs"]
mod support;
#[path = "../benches/latency/unicode_fallback_scaling.rs"]
mod unicode_fallback_scaling;

#[test]
fn all_48_cases_have_distinct_keys_and_independently_checked_outcomes() {
    let normalizer = normalizer_tr::Normalizer::new().unwrap();
    let cases = unicode_fallback_scaling::cases();
    assert_eq!(cases.len(), 48);
    let mut keys = std::collections::BTreeSet::new();
    for case in &cases {
        assert!(keys.insert((
            case.id,
            case.text.len(),
            format!("{:?}", case.options.ambiguity_policy),
        )));
        unicode_fallback_scaling::check(case, &normalizer.normalize(&case.text, &case.options));
    }
}
