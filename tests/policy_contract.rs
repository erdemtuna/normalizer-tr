//! Normalization policy contracts and original-source partition invariants.
use normalizer_tr::{
    AmbiguityPolicy, Hint, HintKind, NormalizeError, NormalizeOptions, Normalizer, SegmentKind,
    SourceRange,
};
use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

#[path = "support/policy_contract.rs"]
mod policy_fixtures;

#[test]
fn normalization_cases_have_exact_policy_outcomes_and_source_partitions() {
    let cases: Vec<Value> = policy_fixtures::load().0;
    let normalizer = Normalizer::new().unwrap();
    for case in cases {
        let input = case["text"].as_str().unwrap();
        let mut options = NormalizeOptions::default();
        if let Some(hint) = case.get("hint") {
            assert_eq!(hint["kind"], "date");
            options.hints.push(Hint::new(
                SourceRange::new(
                    hint["start"].as_u64().unwrap() as usize,
                    hint["end"].as_u64().unwrap() as usize,
                ),
                HintKind::Date,
            ));
        }

        let preserve = normalizer.normalize(input, &options).unwrap();
        for policy in [
            AmbiguityPolicy::Preserve,
            AmbiguityPolicy::Reject,
            AmbiguityPolicy::Fallback,
        ] {
            options.ambiguity_policy = policy;
            let result = normalizer.normalize(input, &options);
            if case.get("category").is_some() && policy == AmbiguityPolicy::Reject {
                assert_eq!(
                    result,
                    Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
                );
                continue;
            }
            let result = result.unwrap();
            let expected = if case.get("category").is_some() {
                if policy == AmbiguityPolicy::Fallback {
                    case["fallback"].as_str().unwrap()
                } else {
                    input
                }
            } else {
                case["expected"].as_str().unwrap()
            };
            assert_eq!(
                result.normalized_text(),
                expected,
                "{} {policy:?}",
                case["id"]
            );
            assert_eq!(
                result.complete(),
                case.get("category").is_none() || policy == AmbiguityPolicy::Fallback
            );
            if let Some(category) = case["category"].as_str() {
                if policy == AmbiguityPolicy::Fallback {
                    assert_eq!(result.fallbacks().len(), 1, "{}", case["id"]);
                    let record = &result.fallbacks()[0];
                    assert_eq!(record.range(), SourceRange::new(0, input.len()));
                    assert_eq!(
                        format!("{:?}", record.attempted_class()),
                        case["fallback_class"].as_str().unwrap()
                    );
                    assert_eq!(
                        format!("{:?}", record.strategy()),
                        case["strategy"].as_str().unwrap()
                    );
                    assert_eq!(
                        format!("{:?}", record.original_category().unwrap()),
                        category
                    );
                } else {
                    assert_eq!(result.issues().len(), 1, "{}", case["id"]);
                    assert_eq!(result.issues()[0].range(), SourceRange::new(0, input.len()));
                    assert_eq!(format!("{:?}", result.issues()[0].category()), category);
                }
            } else {
                assert!(result.issues().is_empty() && result.fallbacks().is_empty());
                assert_eq!(result, preserve);
                if let Some(kind) = case["kind"].as_str() {
                    assert_eq!(result.segments().len(), 1, "{}", case["id"]);
                    assert_eq!(format!("{:?}", result.segments()[0].kind()), kind);
                }
            }
            let mut boundaries: Vec<_> = input.grapheme_indices(true).map(|(i, _)| i).collect();
            boundaries.push(input.len());
            let mut cursor = 0;
            for segment in result.segments() {
                assert_eq!(segment.range().start(), cursor);
                cursor = segment.range().end();
                assert!(boundaries.contains(&cursor));
                if matches!(
                    segment.kind(),
                    SegmentKind::Verbatim | SegmentKind::Unresolved
                ) {
                    assert_eq!(segment.text(), &input[segment.range().start()..cursor]);
                }
            }
            assert_eq!(cursor, input.len());
            assert_eq!(
                result
                    .segments()
                    .iter()
                    .map(|s| s.text())
                    .collect::<String>(),
                expected
            );
        }
    }
}

#[test]
fn compound_hints_cannot_cut_expressions_in_any_policy() {
    let n = Normalizer::new().unwrap();
    for policy in [
        AmbiguityPolicy::Preserve,
        AmbiguityPolicy::Reject,
        AmbiguityPolicy::Fallback,
    ] {
        for (text, end) in [
            ("1 234,50 TL", 1),
            ("25TL", 2),
            ("12,5%", 4),
            ("% 12,5'lik", 1),
            ("10 - 15 kişi", 2),
            ("14/03/2026", 2),
            ("5°C", 1),
        ] {
            assert_eq!(
                n.normalize(
                    text,
                    &NormalizeOptions {
                        ambiguity_policy: policy,
                        hints: vec![Hint::new(SourceRange::new(0, end), HintKind::Cardinal)],
                    },
                ),
                Err(NormalizeError::InvalidHint),
                "{text} {policy:?}"
            );
        }
    }
}

#[test]
fn source_notation_preserves_nfd_hint_coordinates_and_address_punctuation() {
    let n = Normalizer::new().unwrap();
    let text = "o\u{308} 03/04/2026";
    for policy in [
        AmbiguityPolicy::Preserve,
        AmbiguityPolicy::Reject,
        AmbiguityPolicy::Fallback,
    ] {
        let result = n
            .normalize(
                text,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    hints: vec![Hint::new(SourceRange::new(4, text.len()), HintKind::Date)],
                },
            )
            .unwrap();
        assert_eq!(
            result.normalized_text(),
            "o\u{308} üç Nisan iki bin yirmi altı"
        );
        assert_eq!(
            result.segments()[1].range(),
            SourceRange::new(4, text.len())
        );
        let result = n
            .normalize(
                "https://ornek.com/a,b;c?x=25TL&y=5kg",
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(result.complete());
        assert_eq!(
            result.normalized_text(),
            "ha te te pe es iki nokta eğik çizgi eğik çizgi ornek nokta kom eğik çizgi a virgül b noktalı virgül c soru işareti x eşittir iki beş TL ve y eşittir beş kg"
        );
        assert_eq!(result.segments().len(), 1);
        assert_eq!(result.segments()[0].kind(), SegmentKind::Electronic);
    }
}

#[test]
fn quantity_like_identifier_interiors_do_not_become_primary_readings() {
    let n = Normalizer::new().unwrap();
    for source in ["AB12(25kg)", "AB12('25kg')", "AB12[25TL]"] {
        let result = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(result.normalized_text(), source);
        assert!(!result.complete());
        assert!(result.segments().iter().all(|segment| matches!(
            segment.kind(),
            SegmentKind::Verbatim | SegmentKind::Unresolved
        )));
    }
    let result = n
        .normalize("AB12(25kg);30kg", &NormalizeOptions::default())
        .unwrap();
    assert_eq!(result.normalized_text(), "AB12(25kg);otuz kilogram");
    assert!(!result.complete());
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(128))]
    #[test]
    fn independent_cases_keep_policy_boundaries(indices in proptest::collection::vec(proptest::prelude::any::<usize>(), 1..8)) {
        let cases: Vec<Value> = policy_fixtures::load().0;
        let selected: Vec<_> = indices.iter().map(|index| &cases[index % cases.len()]).filter(|case| case.get("hint").is_none()).collect();
        proptest::prop_assume!(!selected.is_empty());
        let source = selected.iter().map(|case| case["text"].as_str().unwrap()).collect::<Vec<_>>().join(" ; ");
        let normalizer = Normalizer::new().unwrap();
        let preserve = normalizer.normalize(&source, &NormalizeOptions::default()).unwrap();
        let expected = selected.iter().map(|case| case.get("expected").unwrap_or(&case["text"]).as_str().unwrap()).collect::<Vec<_>>().join(" ; ");
        proptest::prop_assert_eq!(preserve.normalized_text(), expected);
        let reject = normalizer.normalize(&source, &NormalizeOptions { ambiguity_policy: AmbiguityPolicy::Reject, ..Default::default() });
        if preserve.complete() {
            proptest::prop_assert_eq!(reject.unwrap(), preserve);
        } else {
            proptest::prop_assert_eq!(reject, Err(NormalizeError::Unresolved(preserve.issues().to_vec())));
        }
        let fallback = normalizer.normalize(&source, &NormalizeOptions { ambiguity_policy: AmbiguityPolicy::Fallback, ..Default::default() }).unwrap();
        let expected = selected.iter().map(|case| case.get("expected").unwrap_or(&case["fallback"]).as_str().unwrap()).collect::<Vec<_>>().join(" ; ");
        proptest::prop_assert_eq!(fallback.normalized_text(), expected);
        proptest::prop_assert!(fallback.complete() && fallback.issues().is_empty());
    }
}
