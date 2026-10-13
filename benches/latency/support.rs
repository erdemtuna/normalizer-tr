//! Shared development evidence; preparation and serialization are never timed.
use std::{
    collections::BTreeSet,
    hint::black_box,
    io::{self, BufWriter, Write},
    time::Instant,
};

use normalizer_tr::{
    AmbiguityPolicy, Hint, HintKind, LimitKind, MAX_HINTS, MAX_INPUT_BYTES, NormalizeError,
    NormalizeOptions, NormalizeResult, Normalizer, SourceRange,
};
use serde_json::{Value, json};

pub(super) const SCALING_SAMPLES: usize = 100;
pub(super) const LONG_METHOD: &str = "instant-full-call-disposal-100-no-warmup-v1";

pub(super) fn write_report(writer: impl Write, report: &Value) -> io::Result<()> {
    let mut writer = BufWriter::new(writer);
    serde_json::to_writer_pretty(&mut writer, report)?;
    writer.flush()
}

pub(super) struct ScalingCase {
    pub(super) id: &'static str,
    pub(super) text: String,
    pub(super) options: NormalizeOptions,
}

pub(super) fn policy_name(policy: AmbiguityPolicy) -> &'static str {
    match policy {
        AmbiguityPolicy::Preserve => "preserve",
        AmbiguityPolicy::Reject => "reject",
        AmbiguityPolicy::Fallback => "fallback",
    }
}

pub(super) fn input_fingerprint(text: &str, options: &NormalizeOptions) -> Value {
    let hints: Vec<_> = options
        .hints
        .iter()
        .map(|hint| json!({"range": hint.range(), "kind": hint.kind()}))
        .collect();
    json!({
        "text": text,
        "options": {"ambiguity_policy": options.ambiguity_policy, "hints": hints},
    })
}

pub(super) fn outcome(result: Result<NormalizeResult, NormalizeError>) -> Value {
    match result {
        Ok(result) => json!({"result": result}),
        Err(NormalizeError::Unresolved(issues)) => {
            json!({"error": "Unresolved", "issues": issues})
        }
        Err(NormalizeError::LimitExceeded(kind)) => {
            json!({"error": "LimitExceeded", "limit": format!("{kind:?}")})
        }
        Err(error) => json!({"error": format!("{error:?}")}),
    }
}

pub(super) fn quantiles(values: &mut [u64]) -> Value {
    assert!(!values.is_empty());
    values.sort_unstable();
    let at = |percent: usize| values[(values.len() * percent).div_ceil(100).saturating_sub(1)];
    json!({
        "count": values.len(), "p50_ns": at(50), "p95_ns": at(95),
        "p99_ns": at(99), "max_ns": values[values.len() - 1],
    })
}

pub(super) fn scaling_row(
    suite: &str,
    id: &str,
    text: &str,
    options: &NormalizeOptions,
    timing: Value,
    expected: Value,
) -> Value {
    json!({
        "suite": suite, "id": id, "bytes": text.len(),
        "policy": policy_name(options.ambiguity_policy),
        "method": LONG_METHOD, "input_fingerprint": input_fingerprint(text, options),
        "timing": timing, "outcome": expected,
    })
}

pub(super) fn measure_cases(
    normalizer: &Normalizer,
    suite: &'static str,
    cases: Vec<ScalingCase>,
    check: fn(&ScalingCase, &Result<NormalizeResult, NormalizeError>),
) -> Vec<Value> {
    cases
        .into_iter()
        .map(|case| {
            let expected = normalizer.normalize(&case.text, &case.options);
            check(&case, &expected);
            let mut samples = Vec::with_capacity(SCALING_SAMPLES);
            for _ in 0..SCALING_SAMPLES {
                let start = Instant::now();
                drop(black_box(
                    normalizer.normalize(black_box(&case.text), black_box(&case.options)),
                ));
                samples.push(start.elapsed().as_nanos() as u64);
            }
            let after = normalizer.normalize(&case.text, &case.options);
            check(&case, &after);
            assert_eq!(after, expected, "{}:{}", suite, case.id);
            scaling_row(
                suite,
                case.id,
                &case.text,
                &case.options,
                quantiles(&mut samples),
                outcome(expected),
            )
        })
        .collect()
}

pub(super) fn resource_cases() -> Vec<ScalingCase> {
    let mut cases = Vec::new();
    for ambiguity_policy in [
        AmbiguityPolicy::Preserve,
        AmbiguityPolicy::Reject,
        AmbiguityPolicy::Fallback,
    ] {
        cases.push(ScalingCase {
            id: "input-over-limit",
            text: "a".repeat(MAX_INPUT_BYTES + 1),
            options: NormalizeOptions {
                ambiguity_policy,
                ..Default::default()
            },
        });
        cases.push(ScalingCase {
            id: "hints-over-limit",
            text: "1;".repeat(MAX_HINTS + 1),
            options: NormalizeOptions {
                ambiguity_policy,
                hints: (0..=MAX_HINTS)
                    .map(|i| Hint::new(SourceRange::new(i * 2, i * 2 + 1), HintKind::Cardinal))
                    .collect(),
            },
        });
    }
    cases
}

pub(super) fn check_resource(case: &ScalingCase, result: &Result<NormalizeResult, NormalizeError>) {
    let limit = match case.id {
        "input-over-limit" => LimitKind::Input,
        "hints-over-limit" => LimitKind::Hints,
        other => panic!("unknown resource workload: {other}"),
    };
    assert_eq!(*result, Err(NormalizeError::LimitExceeded(limit)));
}

pub(super) fn inventory_keys(inventory: &Value) -> BTreeSet<(String, String, usize, String)> {
    let mut keys = BTreeSet::new();
    for group in inventory["native"].as_array().unwrap() {
        for id in group["ids"].as_array().unwrap() {
            for bytes in group["bytes"].as_array().unwrap() {
                for policy in group["policies"].as_array().unwrap() {
                    let key = (
                        group["suite"].as_str().unwrap().to_owned(),
                        id.as_str().unwrap().to_owned(),
                        usize::try_from(bytes.as_u64().unwrap()).unwrap(),
                        policy.as_str().unwrap().to_owned(),
                    );
                    assert!(keys.insert(key), "duplicate required workload");
                }
            }
        }
    }
    keys
}

pub(super) fn validate_prepared(inventory: &Value, suites: &[(&str, &[ScalingCase])]) {
    let expected = inventory_keys(inventory);
    let mut actual = BTreeSet::new();
    for (suite, cases) in suites {
        for case in *cases {
            let key = (
                (*suite).to_owned(),
                case.id.to_owned(),
                case.text.len(),
                policy_name(case.options.ambiguity_policy).to_owned(),
            );
            assert!(actual.insert(key), "duplicate prepared workload");
        }
    }
    assert_eq!(
        actual, expected,
        "prepared workload coverage differs before timing"
    );
}

pub(super) fn validate_inventory(report: &Value) {
    let expected = inventory_keys(&report["workload_inventory"]);
    let mut actual = BTreeSet::new();
    for section in [
        &report["expanded_coverage_measurement"]["scaling"],
        &report["large"],
        &report["limit_diagnostics"],
        &report["reviewed_recognition_scaling"]["scaling"],
        &report["supplementary_scaling"],
    ] {
        for row in section.as_array().unwrap() {
            let key = (
                row["suite"].as_str().unwrap().to_owned(),
                row["id"].as_str().unwrap().to_owned(),
                usize::try_from(row["bytes"].as_u64().unwrap()).unwrap(),
                row["policy"].as_str().unwrap().to_owned(),
            );
            assert!(actual.insert(key), "duplicate measured workload");
        }
    }
    assert_eq!(actual, expected, "measured workload coverage differs");
}
