//! Dev-only checked per-call latency sampler; never a product CLI.
use std::{collections::BTreeMap, hint::black_box, time::Instant};

use normalizer_tr::{AmbiguityPolicy, Hint, NormalizeOptions, Normalizer, SourceRange};
use serde_json::{Value, json};

#[path = "latency/legacy_inputs.rs"]
mod legacy_inputs;
#[path = "../tests/support/policy_contract.rs"]
mod policy_fixtures;
#[path = "latency/recognition_scaling.rs"]
mod recognition_scaling;
#[path = "latency/scanner_scaling.rs"]
mod scanner_scaling;
#[path = "latency/source_mapping.rs"]
mod source_mapping;
#[path = "latency/support.rs"]
mod support;
#[path = "latency/unicode_fallback_scaling.rs"]
mod unicode_fallback_scaling;

use support::{input_fingerprint, outcome, quantiles, scaling_row};

fn options(case: &Value, policy: AmbiguityPolicy) -> NormalizeOptions {
    let mut options = NormalizeOptions {
        ambiguity_policy: policy,
        ..Default::default()
    };
    if let Some(hint) = policy_fixtures::hint(case) {
        options.hints.push(hint);
    }
    options
}

fn fallback_measurements(normalizer: &Normalizer, corpus: &[Value]) -> Value {
    let mut cohorts = BTreeMap::new();
    let mut classes = BTreeMap::<String, Vec<u64>>::new();
    let mut snapshots = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    for cohort in ["short", "medium"] {
        let prepared: Vec<_> = corpus
            .iter()
            .filter(|case| case["cohort"] == cohort)
            .map(|case| {
                let text = case["text"].as_str().unwrap();
                let options = options(case, AmbiguityPolicy::Fallback);
                let expected = outcome(normalizer.normalize(text, &options));
                snapshots.insert(case["id"].as_str().unwrap(), expected.clone());
                inputs.insert(
                    case["id"].as_str().unwrap(),
                    input_fingerprint(text, &options),
                );
                (text, options, expected, case["class"].as_str().unwrap())
            })
            .collect();
        for index in 0..2000 {
            let (text, options, _, _) = &prepared[index % prepared.len()];
            drop(black_box(
                normalizer.normalize(black_box(text), black_box(options)),
            ));
        }
        let mut samples = Vec::with_capacity(10000);
        for index in 0..10000 {
            let (text, options, expected, class) = &prepared[index % prepared.len()];
            if index < prepared.len() {
                assert_eq!(outcome(normalizer.normalize(text, options)), *expected);
            }
            let start = Instant::now();
            drop(black_box(
                normalizer.normalize(black_box(text), black_box(options)),
            ));
            let elapsed = start.elapsed().as_nanos() as u64;
            samples.push(elapsed);
            classes
                .entry(format!("{cohort}:{class}:fallback"))
                .or_default()
                .push(elapsed);
        }
        for (text, options, expected, _) in &prepared {
            assert_eq!(outcome(normalizer.normalize(text, options)), *expected);
        }
        cohorts.insert(cohort, quantiles(&mut samples));
    }
    let per_class: BTreeMap<_, _> = classes
        .into_iter()
        .map(|(class, mut samples)| (class, quantiles(&mut samples)))
        .collect();
    json!({"cohorts":cohorts, "per_class_policy":per_class, "snapshots":snapshots,"inputs":inputs,
        "method":"fallback only; same frozen cohorts; 2000 warmup and 10000 individually timed calls per cohort; no filtering or overhead subtraction"})
}

fn check_policy_contract(
    case: &Value,
    policy: AmbiguityPolicy,
    prefix: &str,
    result: &Result<normalizer_tr::NormalizeResult, normalizer_tr::NormalizeError>,
) {
    let unresolved = case.get("category").is_some();
    match result {
        Err(normalizer_tr::NormalizeError::Unresolved(issues))
            if unresolved && policy == AmbiguityPolicy::Reject =>
        {
            assert_eq!(issues.len(), 1, "{}", case["id"]);
            assert_eq!(
                format!("{:?}", issues[0].category()),
                case["category"].as_str().unwrap()
            );
        }
        Ok(result) => {
            let expected = if unresolved {
                if policy == AmbiguityPolicy::Fallback {
                    &case["fallback"]
                } else {
                    &case["text"]
                }
            } else {
                &case["expected"]
            };
            assert_eq!(
                result.normalized_text(),
                format!("{prefix}{}", expected.as_str().unwrap()),
                "{}",
                case["id"]
            );
            assert_eq!(
                result.complete(),
                !unresolved || policy == AmbiguityPolicy::Fallback
            );
            assert_eq!(
                result.fallback_used(),
                unresolved && policy == AmbiguityPolicy::Fallback
            );
        }
        other => panic!(
            "policy-contract case {} has unexpected outcome: {other:?}",
            case["id"]
        ),
    }
}

fn policy_contract_measurements(
    normalizer: &Normalizer,
    scaling_inputs: Vec<support::ScalingCase>,
) -> Value {
    let (cases, input_bytes) = policy_fixtures::load();
    let mut cohorts = BTreeMap::new();
    let mut classes = BTreeMap::<String, Vec<u64>>::new();
    let mut snapshots = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    for cohort in ["short", "medium"] {
        let prefix = if cohort == "medium" {
            "Bu sentetik paragraf kaynak metnin ve miktarların birlikte okunmasını kontrol eder. "
                .repeat(4)
        } else {
            String::new()
        };
        for (name, policy) in [
            ("preserve", AmbiguityPolicy::Preserve),
            ("reject", AmbiguityPolicy::Reject),
            ("fallback", AmbiguityPolicy::Fallback),
        ] {
            let prepared: Vec<_> = cases
                .iter()
                .map(|case| {
                    let text = format!("{prefix}{}", case["text"].as_str().unwrap());
                    assert!(if cohort == "short" {
                        text.len() <= 256
                    } else {
                        (257..=1024).contains(&text.len())
                    });
                    let mut options = options(case, policy);
                    for hint in &mut options.hints {
                        let range = hint.range();
                        *hint = Hint::new(
                            SourceRange::new(
                                range.start() + prefix.len(),
                                range.end() + prefix.len(),
                            ),
                            hint.kind(),
                        );
                    }
                    let expected = normalizer.normalize(&text, &options);
                    check_policy_contract(case, policy, &prefix, &expected);
                    let key = format!("{cohort}:{}:{name}", case["id"].as_str().unwrap());
                    inputs.insert(key.clone(), input_fingerprint(&text, &options));
                    snapshots.insert(key, outcome(expected));
                    (case, text, options)
                })
                .collect();
            for index in 0..2000 {
                let (_, text, options) = &prepared[index % prepared.len()];
                drop(black_box(
                    normalizer.normalize(black_box(text), black_box(options)),
                ));
            }
            let mut samples = Vec::with_capacity(10000);
            for index in 0..10000 {
                let (case, text, options) = &prepared[index % prepared.len()];
                let start = Instant::now();
                drop(black_box(
                    normalizer.normalize(black_box(text), black_box(options)),
                ));
                let elapsed = start.elapsed().as_nanos() as u64;
                samples.push(elapsed);
                classes
                    .entry(format!(
                        "{cohort}:{}:{name}",
                        case["class"].as_str().unwrap()
                    ))
                    .or_default()
                    .push(elapsed);
            }
            for (case, text, options) in &prepared {
                let after = normalizer.normalize(text, options);
                check_policy_contract(case, policy, &prefix, &after);
                let key = format!("{cohort}:{}:{name}", case["id"].as_str().unwrap());
                assert_eq!(outcome(after), snapshots[&key]);
            }
            cohorts.insert(format!("{cohort}:{name}"), quantiles(&mut samples));
        }
    }
    let mut scaling = Vec::new();
    for support::ScalingCase { id, text, options } in scaling_inputs {
        let expected = outcome(normalizer.normalize(&text, &options));
        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = Instant::now();
            drop(black_box(normalizer.normalize(black_box(&text), &options)));
            samples.push(start.elapsed().as_nanos() as u64);
        }
        assert_eq!(outcome(normalizer.normalize(&text, &options)), expected);
        scaling.push(scaling_row(
            "expanded",
            id,
            &text,
            &options,
            quantiles(&mut samples),
            expected,
        ));
    }
    let per_class: BTreeMap<_, _> = classes
        .into_iter()
        .map(|(key, mut values)| (key, quantiles(&mut values)))
        .collect();
    json!({"input_source":"tests/fixtures/policy-contract.json","input_bytes":input_bytes,"input_cases":cases,
        "method":"separate policy cohorts; reviewed goldens before/after; 2000 warmups and 10000 calls including disposal; all outliers retained",
        "cohorts":cohorts,"per_class_policy":per_class,"scaling":scaling,
        "inputs":inputs,"snapshots":snapshots})
}

fn main() {
    let source_sha = source_revision();
    // Cargo supplies --bench even when the target uses its own timing harness.
    let args: Vec<_> = std::env::args().filter(|arg| arg != "--bench").collect();
    let output = match args.as_slice() {
        [_, flag, output] if flag == "--output" => output,
        _ => panic!("usage: latency --output <new-report.json>"),
    };
    let mut corpus: Vec<Value> = serde_json::from_str(include_str!("corpus.json")).unwrap();
    corpus.extend(serde_json::from_str::<Vec<Value>>(include_str!("intent-corpus.json")).unwrap());
    let inventory: Value =
        serde_json::from_str(include_str!("latency/workload_inventory.json")).unwrap();
    let expanded_inputs = legacy_inputs::expanded();
    let large_inputs = legacy_inputs::large();
    let limit_inputs = legacy_inputs::limits();
    let recognition_inputs = recognition_scaling::cases();
    let scanner_inputs = scanner_scaling::cases();
    let mapping_inputs = source_mapping::cases();
    let unicode_inputs = unicode_fallback_scaling::cases();
    let resource_inputs = support::resource_cases();
    support::validate_prepared(
        &inventory,
        &[
            ("expanded", &expanded_inputs),
            ("large", &large_inputs),
            ("limits", &limit_inputs),
            ("recognition", &recognition_inputs),
            ("scanner", &scanner_inputs),
            ("source-map", &mapping_inputs),
            ("unicode-fallback", &unicode_inputs),
            ("resources", &resource_inputs),
        ],
    );
    let first_start = Instant::now();
    drop(black_box(Normalizer::new().unwrap()));
    let first_init_ns = first_start.elapsed().as_nanos() as u64;
    let mut constructors = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        black_box(Normalizer::new().unwrap());
        constructors.push(start.elapsed().as_nanos() as u64);
    }
    let normalizer = Normalizer::new().unwrap();
    let mut snapshots = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    let mut cohorts = BTreeMap::new();
    let mut classes = BTreeMap::<String, Vec<u64>>::new();
    let mut distributions = Vec::new();
    for cohort in ["short", "medium"] {
        let selected: Vec<_> = corpus
            .iter()
            .filter(|case| case["cohort"] == cohort)
            .collect();
        let prepared: Vec<_> = selected
            .iter()
            .flat_map(|case| {
                [false, true].map(|reject| {
                    let text = case["text"].as_str().unwrap();
                    assert!(if cohort == "short" {
                        !text.is_empty() && text.len() <= 256
                    } else {
                        (257..=1024).contains(&text.len())
                    });
                    let options = options(
                        case,
                        if reject {
                            AmbiguityPolicy::Reject
                        } else {
                            AmbiguityPolicy::Preserve
                        },
                    );
                    let expected = outcome(normalizer.normalize(text, &options));
                    let key = format!(
                        "{}:{}",
                        case["id"].as_str().unwrap(),
                        if reject { "reject" } else { "preserve" }
                    );
                    snapshots.insert(key.clone(), expected.clone());
                    inputs.insert(key.clone(), input_fingerprint(text, &options));
                    distributions.push(
                        json!({"id":key,"bytes":text.len(),"cohort":cohort,"class":case["class"]}),
                    );
                    (
                        text,
                        options,
                        expected,
                        case["class"].as_str().unwrap(),
                        reject,
                    )
                })
            })
            .collect();
        for index in 0..2000 {
            let (text, options, _, _, _) = &prepared[index % prepared.len()];
            drop(black_box(
                normalizer.normalize(black_box(text), black_box(options)),
            ));
        }
        let mut samples = Vec::with_capacity(10000);
        for index in 0..10000 {
            let (text, options, expected, class, reject) = &prepared[index % prepared.len()];
            if index < prepared.len() {
                assert_eq!(outcome(normalizer.normalize(text, options)), *expected);
            }
            let start = Instant::now();
            drop(black_box(
                normalizer.normalize(black_box(text), black_box(options)),
            ));
            let elapsed = start.elapsed().as_nanos() as u64;
            samples.push(elapsed);
            classes
                .entry(format!(
                    "{cohort}:{class}:{}",
                    if *reject { "reject" } else { "preserve" }
                ))
                .or_default()
                .push(elapsed);
        }
        for (text, options, expected, _, _) in &prepared {
            assert_eq!(outcome(normalizer.normalize(text, options)), *expected);
        }
        cohorts.insert(cohort, quantiles(&mut samples));
    }
    let per_class: BTreeMap<_, _> = classes
        .into_iter()
        .map(|(key, mut samples)| (key, quantiles(&mut samples)))
        .collect();
    let mut clock = (0..10000)
        .map(|_| {
            let start = Instant::now();
            start.elapsed().as_nanos() as u64
        })
        .collect::<Vec<_>>();
    let mut large = Vec::new();
    for support::ScalingCase { id, text, .. } in large_inputs {
        let mut times = Vec::new();
        let expected = outcome(normalizer.normalize(&text, &NormalizeOptions::default()));
        for _ in 0..100 {
            let start = Instant::now();
            drop(black_box(
                normalizer.normalize(black_box(&text), &NormalizeOptions::default()),
            ));
            times.push(start.elapsed().as_nanos() as u64);
        }
        assert_eq!(
            outcome(normalizer.normalize(&text, &NormalizeOptions::default())),
            expected
        );
        large.push(scaling_row(
            "large",
            id,
            &text,
            &NormalizeOptions::default(),
            quantiles(&mut times),
            expected,
        ));
    }
    let mut controls = Vec::new();
    for support::ScalingCase { id, text, options } in limit_inputs {
        let expected = outcome(normalizer.normalize(&text, &options));
        let mut times = Vec::new();
        for _ in 0..100 {
            let start = Instant::now();
            drop(black_box(
                normalizer.normalize(black_box(&text), black_box(&options)),
            ));
            times.push(start.elapsed().as_nanos() as u64);
        }
        assert_eq!(expected, outcome(normalizer.normalize(&text, &options)));
        controls.push(scaling_row(
            "limits",
            id,
            &text,
            &options,
            quantiles(&mut times),
            expected,
        ));
    }
    let fallback = fallback_measurements(&normalizer, &corpus);
    let expanded = policy_contract_measurements(&normalizer, expanded_inputs);
    let reviewed = recognition_scaling::measure(&normalizer, recognition_inputs);
    let mut supplementary = support::measure_cases(
        &normalizer,
        "scanner",
        scanner_inputs,
        scanner_scaling::check,
    );
    supplementary.extend(support::measure_cases(
        &normalizer,
        "source-map",
        mapping_inputs,
        source_mapping::check,
    ));
    supplementary.extend(support::measure_cases(
        &normalizer,
        "unicode-fallback",
        unicode_inputs,
        unicode_fallback_scaling::check,
    ));
    supplementary.extend(support::measure_cases(
        &normalizer,
        "resources",
        resource_inputs,
        support::check_resource,
    ));
    let report = json!({
        "schema_version":2,"report_kind":"native","method_id":"instant-frozen-cohorts-v1",
        "workload_inventory":inventory,
        "source_sha":source_sha,"normalizer_id":normalizer.normalizer_id(),"package_version":env!("CARGO_PKG_VERSION"),
        "method":{"samples_per_cohort":10000,"warmup_per_cohort":2000,
                  "timing":"Instant per call; validation mapping recognition rendering owned result disposal included",
                  "order":"deterministic corpus order alternating preserve/reject","outliers":"all retained; no overhead subtraction"},
        "constructor_first_process_init_ns":first_init_ns,
        "constructors":quantiles(&mut constructors),
        "constructor_method":"first process init separately; distribution is subsequent same-process constructor calls, not cold starts",
        "cohorts":cohorts,"per_class_policy":per_class,
        "distribution":distributions,"clock_overhead":quantiles(&mut clock),"large":large,"limit_diagnostics":controls,
        "snapshots":snapshots,"inputs":inputs,
        "fallback_measurement":fallback,
        "expanded_coverage_measurement":expanded,
        "reviewed_recognition_scaling":reviewed,
        "supplementary_scaling":supplementary,
    });
    support::validate_inventory(&report);
    assert_eq!(
        source_revision(),
        source_sha,
        "source changed during sampling"
    );
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    support::write_report(file, &report).expect("report write and flush must succeed");
    println!("Saved checked per-call measurements to {output}");
}

fn source_revision() -> String {
    let run = |args: &[&str]| {
        let result = std::process::Command::new("git")
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("git source provenance is required");
        assert!(result.status.success(), "git provenance failed");
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    };
    assert!(
        run(&["status", "--porcelain=v1", "--untracked-files=no"]).is_empty(),
        "sampler requires clean committed tracked sources"
    );
    run(&["rev-parse", "HEAD"])
}
