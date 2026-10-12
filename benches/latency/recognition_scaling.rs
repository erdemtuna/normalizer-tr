//! Additional reviewed adversarial paths, separate from frozen latency cohorts.
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use normalizer_tr::{
    AmbiguityPolicy, LimitKind, MAX_CANDIDATES, NormalizeError, NormalizeOptions, NormalizeResult,
    Normalizer, SegmentKind, SourceRange, WorkControl,
};
use serde_json::{Value, json};

const SAMPLES: usize = 100;
const PHONE_PATTERN: &str = "01 1 ";
const URL_PREFIX: &str = "https://ornek.com/";
const URL_SPEECH: &str =
    "ha te te pe es iki nokta eğik çizgi eğik çizgi ornek nokta kom eğik çizgi";

fn input_fingerprint(input: &str, options: &NormalizeOptions) -> Value {
    let hints: Vec<_> = options
        .hints
        .iter()
        .map(|hint| json!({"range": hint.range(), "kind": hint.kind()}))
        .collect();
    json!({
        "text": input,
        "options": {"ambiguity_policy": options.ambiguity_policy, "hints": hints},
    })
}

fn check_phone(
    result: &Result<NormalizeResult, NormalizeError>,
    count: usize,
    policy: AmbiguityPolicy,
) {
    if count * 2 > MAX_CANDIDATES {
        assert_eq!(
            *result,
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates))
        );
        return;
    }
    let issues = match result {
        Err(NormalizeError::Unresolved(issues)) if policy == AmbiguityPolicy::Reject => issues,
        Ok(result) if policy == AmbiguityPolicy::Fallback => {
            assert!(result.complete());
            assert_eq!(
                result.normalized_text().trim_end(),
                "sıfır bir bir ".repeat(count).trim_end()
            );
            assert_eq!(result.fallbacks().len(), count);
            for (index, issue) in result.fallbacks().iter().enumerate() {
                assert_eq!(
                    issue.range(),
                    SourceRange::new(index * PHONE_PATTERN.len(), index * PHONE_PATTERN.len() + 2)
                );
            }
            return;
        }
        Ok(result) if policy == AmbiguityPolicy::Preserve => {
            assert!(!result.complete());
            assert_eq!(
                result.normalized_text().trim_end(),
                "01 bir ".repeat(count).trim_end()
            );
            result.issues()
        }
        other => panic!("unexpected failed-phone outcome: {other:?}"),
    };
    assert_eq!(issues.len(), count);
    for (index, issue) in issues.iter().enumerate() {
        assert_eq!(
            issue.range(),
            SourceRange::new(index * PHONE_PATTERN.len(), index * PHONE_PATTERN.len() + 2)
        );
    }
}

fn check_url(result: &Result<NormalizeResult, NormalizeError>, input: &str) {
    let result = result.as_ref().unwrap();
    assert!(result.complete() && result.issues().is_empty() && result.fallbacks().is_empty());
    assert_eq!(
        result.normalized_text(),
        format!("{URL_SPEECH}{}", &input[URL_PREFIX.len()..])
    );
    assert_eq!(result.segments()[0].kind(), SegmentKind::Electronic);
    assert_eq!(
        result.segments()[0].range(),
        SourceRange::new(0, URL_PREFIX.len())
    );
}

pub(super) fn measure(normalizer: &Normalizer) -> Value {
    let mut scaling = Vec::new();
    for (path, sizes) in [
        ("failed-telephone", [1024, 4096, 8192, 32768]),
        (
            "electronic-trailing-parentheses",
            [4096, 8192, 16384, 32768],
        ),
    ] {
        for bytes in sizes {
            let input = if path == "failed-telephone" {
                let mut input = PHONE_PATTERN.repeat(bytes / PHONE_PATTERN.len());
                input.push_str(&" ".repeat(bytes - input.len()));
                input
            } else {
                format!("{URL_PREFIX}{}", ")".repeat(bytes - URL_PREFIX.len()))
            };
            assert_eq!(input.len(), bytes);
            for (name, policy) in [
                ("preserve", AmbiguityPolicy::Preserve),
                ("reject", AmbiguityPolicy::Reject),
                ("fallback", AmbiguityPolicy::Fallback),
            ] {
                let options = NormalizeOptions {
                    ambiguity_policy: policy,
                    ..Default::default()
                };
                let expected = normalizer.normalize(&input, &options);
                if path == "failed-telephone" {
                    check_phone(&expected, bytes / PHONE_PATTERN.len(), policy);
                } else {
                    check_url(&expected, &input);
                }
                let mut samples = Vec::with_capacity(SAMPLES);
                for _ in 0..SAMPLES {
                    let start = Instant::now();
                    drop(black_box(
                        normalizer.normalize(black_box(&input), black_box(&options)),
                    ));
                    samples.push(start.elapsed().as_nanos() as u64);
                }
                assert_eq!(normalizer.normalize(&input, &options), expected);
                let fingerprint = input_fingerprint(&input, &options);
                scaling.push(json!({
                    "path":path,"bytes":bytes,"policy":name,
                    "input_fingerprint":fingerprint,
                    "timing":super::quantiles(&mut samples),"outcome":super::outcome(expected),
                }));
            }
        }
    }
    let input = format!("{URL_PREFIX}{}", ")".repeat(32768 - URL_PREFIX.len()));
    let options = NormalizeOptions::default();
    let mut deadlines = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let control = WorkControl::new(Some(start + Duration::from_millis(10)));
        let result = normalizer.normalize_controlled(&input, &options, &control);
        let elapsed = start.elapsed().as_nanos() as u64;
        if !matches!(result, Err(NormalizeError::Cancelled)) {
            check_url(&result, &input);
        }
        deadlines.push(json!({"elapsed_ns":elapsed,"outcome":super::outcome(result)}));
    }
    json!({
        "method":"exact input sizes; 100 individual release calls including disposal; checked independent text, diagnostics and limits before/after; no timing assertion",
        "scaling":scaling,"electronic_deadline_10ms":deadlines,
    })
}
