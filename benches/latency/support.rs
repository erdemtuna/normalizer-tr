//! Shared development evidence; preparation and serialization are never timed.
use std::{hint::black_box, time::Instant};

use normalizer_tr::{
    AmbiguityPolicy, NormalizeError, NormalizeOptions, NormalizeResult, Normalizer,
};
use serde_json::{Value, json};

pub(super) const SCALING_SAMPLES: usize = 100;
pub(super) const LONG_METHOD: &str = "instant-full-call-disposal-100-no-warmup-v1";

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
