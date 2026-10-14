//! Atomic failed telephone owners, independent neighbors and original coordinates.
use std::time::{Duration, Instant};

use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, Hint, HintKind,
    IssueCategory, LimitKind, MAX_INPUT_BYTES, NormalizeError, NormalizeOptions, NormalizeResult,
    Normalizer, SegmentKind, SourceRange, WorkControl,
};
use unicode_segmentation::UnicodeSegmentation;

const PHONE: &str = "0532 123 45 67'den";
const LITERAL: &str = "sıfır beş üç iki bir iki üç dört beş altı yedi kesme de e ne";
const SPOKEN: &str = "sıfır beş yüz otuz iki yüz yirmi üç kırk beş altmış yedi";
const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

fn options(policy: AmbiguityPolicy) -> NormalizeOptions {
    NormalizeOptions {
        ambiguity_policy: policy,
        ..Default::default()
    }
}

fn partition(input: &str, result: &NormalizeResult) {
    let boundaries: Vec<_> = input
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(std::iter::once(input.len()))
        .collect();
    let mut cursor = 0;
    for segment in result.segments() {
        assert_eq!(segment.range().start(), cursor);
        assert!(segment.range().end() > cursor);
        cursor = segment.range().end();
        assert!(boundaries.contains(&cursor));
        if matches!(
            segment.kind(),
            SegmentKind::Verbatim | SegmentKind::Unresolved
        ) {
            assert_eq!(
                segment.text(),
                &input[segment.range().start()..segment.range().end()]
            );
        }
    }
    assert_eq!(cursor, input.len());
    assert_eq!(
        result
            .segments()
            .iter()
            .map(|s| s.text())
            .collect::<String>(),
        result.normalized_text()
    );
}

fn failure(
    normalizer: &Normalizer,
    source: &str,
    range: SourceRange,
) -> (NormalizeResult, NormalizeResult) {
    failure_with_strategy(normalizer, source, range, FallbackStrategy::Literal)
}

fn failure_with_strategy(
    normalizer: &Normalizer,
    source: &str,
    range: SourceRange,
    strategy: FallbackStrategy,
) -> (NormalizeResult, NormalizeResult) {
    let preserved = normalizer.normalize(source, &Default::default()).unwrap();
    assert!(!preserved.complete());
    assert!(preserved.fallbacks().is_empty() && !preserved.fallback_used());
    assert_eq!(preserved.issues().len(), 1, "{source}");
    let issue = &preserved.issues()[0];
    assert_eq!(issue.range(), range);
    assert_eq!(issue.category(), IssueCategory::InvalidExpression);
    assert_eq!(
        issue.explanation(),
        "expression has an invalid value, grammar, or suffix"
    );
    assert_eq!(
        normalizer.normalize(source, &options(AmbiguityPolicy::Reject)),
        Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
    );
    let fallback = normalizer
        .normalize(source, &options(AmbiguityPolicy::Fallback))
        .unwrap();
    assert!(fallback.complete() && fallback.issues().is_empty() && fallback.fallback_used());
    assert_eq!(fallback.fallbacks().len(), 1);
    let diagnostic = &fallback.fallbacks()[0];
    assert_eq!(diagnostic.range(), range);
    assert_eq!(diagnostic.attempted_class(), FallbackClass::Identifier);
    assert_eq!(diagnostic.reason(), FallbackReason::InvalidForm);
    assert_eq!(diagnostic.original_category(), Some(issue.category()));
    assert_eq!(diagnostic.strategy(), strategy);
    for (result, kind, rule) in [
        (&preserved, SegmentKind::Unresolved, "source.unresolved"),
        (&fallback, SegmentKind::Fallback, "source.fallback"),
    ] {
        let owned: Vec<_> = result
            .segments()
            .iter()
            .filter(|segment| {
                segment.range().start() < range.end() && range.start() < segment.range().end()
            })
            .collect();
        assert_eq!(owned.len(), 1);
        assert_eq!(
            (owned[0].range(), owned[0].kind(), owned[0].rule_id()),
            (range, kind, rule)
        );
        partition(source, result);
    }
    (preserved, fallback)
}

#[test]
fn isolated_failed_owners_have_full_policy_provenance() {
    let normalizer = Normalizer::new().unwrap();
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "fixtures/policy-contract/telephone-ownership.json"
    ))
    .unwrap();
    for case in cases {
        if case["fallback_class"] != "Identifier" {
            continue;
        }
        let source = case["text"].as_str().unwrap();
        let strategy = if case["strategy"] == "UnicodeCodePoint" {
            FallbackStrategy::UnicodeCodePoint
        } else {
            FallbackStrategy::Literal
        };
        let (preserved, fallback) = failure_with_strategy(
            &normalizer,
            source,
            SourceRange::new(0, source.len()),
            strategy,
        );
        assert_eq!(preserved.normalized_text(), source);
        assert_eq!(
            fallback.normalized_text(),
            case["fallback"].as_str().unwrap()
        );
        assert_eq!(preserved.segments().len(), 1);
        assert_eq!(fallback.segments().len(), 1);
    }
}

#[test]
fn original_prefixes_quotes_and_following_quantities_keep_exact_segments() {
    let normalizer = Normalizer::new().unwrap();
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "fixtures/policy-contract/telephone-ownership.json"
    ))
    .unwrap();
    for case in cases.iter().filter(|case| {
        [
            "phone-suffixed-national-owner",
            "phone-suffixed-mixed-hyphen",
            "phone-suffixed-adjoining-parentheses",
            "phone-suffixed-parenthesized-final-group",
            "phone-suffixed-parenthesized-tail",
            "phone-suffixed-noncomposing-mark",
        ]
        .contains(&case["id"].as_str().unwrap())
    }) {
        let phone = case["text"].as_str().unwrap();
        let literal = case["fallback"].as_str().unwrap();
        let strategy = if case["strategy"] == "UnicodeCodePoint" {
            FallbackStrategy::UnicodeCodePoint
        } else {
            FallbackStrategy::Literal
        };
        for (prefix, closing) in [
            ("", ""),
            ("O\u{308}nce ", ""),
            ("ö\r\n", ""),
            ("tel ", ""),
            ("telefon ", ""),
            ("'", "'"),
            ("‘", "’"),
            ("(", ")"),
            ("O\u{308}nce (", ")"),
        ] {
            let source = format!("{prefix}{phone}{closing}; 5 kg");
            let owner = SourceRange::new(prefix.len(), prefix.len() + phone.len());
            let (preserved, fallback) =
                failure_with_strategy(&normalizer, &source, owner, strategy);
            assert_eq!(
                preserved.normalized_text(),
                format!("{prefix}{phone}{closing}; beş kilogram")
            );
            assert_eq!(
                fallback.normalized_text(),
                format!("{prefix}{literal}{closing}; beş kilogram")
            );
            for (result, kind, rule, text) in [
                (
                    &preserved,
                    SegmentKind::Unresolved,
                    "source.unresolved",
                    phone,
                ),
                (&fallback, SegmentKind::Fallback, "source.fallback", literal),
            ] {
                let mut expected = Vec::new();
                if !prefix.is_empty() {
                    expected.push((
                        0,
                        prefix.len(),
                        SegmentKind::Verbatim,
                        prefix,
                        "source.verbatim",
                    ));
                }
                let quantity = owner.end() + closing.len() + "; ".len();
                expected.extend([
                    (owner.start(), owner.end(), kind, text, rule),
                    (
                        owner.end(),
                        quantity,
                        SegmentKind::Verbatim,
                        &source[owner.end()..quantity],
                        "source.verbatim",
                    ),
                    (
                        quantity,
                        source.len(),
                        SegmentKind::Unit,
                        "beş kilogram",
                        "quantity",
                    ),
                ]);
                let actual: Vec<_> = result
                    .segments()
                    .iter()
                    .map(|s| {
                        (
                            s.range().start(),
                            s.range().end(),
                            s.kind(),
                            s.text(),
                            s.rule_id(),
                        )
                    })
                    .collect();
                assert_eq!(actual, expected);
            }
        }
    }
    let source = "tel 532 123 45 67'den";
    let (preserved, fallback) = failure(&normalizer, source, SourceRange::new(4, source.len()));
    assert_eq!(preserved.normalized_text(), source);
    assert_eq!(
        fallback.normalized_text(),
        "tel beş üç iki bir iki üç dört beş altı yedi kesme de e ne"
    );
}

#[test]
fn terminal_ownership_does_not_continue_into_independent_fields() {
    let normalizer = Normalizer::new().unwrap();
    for (prefix, source_tail, spoken_tail) in [
        ("", " 89", " seksen dokuz"),
        ("", " 5 kg", " beş kilogram"),
        ("", " 5'ten", " beşten"),
        ("", " adres No:5 Kat:3", " adres numara beş kat üç"),
        ("adres No:5 ", " Kat:3", " kat üç"),
    ] {
        let source = format!("{prefix}{PHONE}{source_tail}");
        let owner = SourceRange::new(prefix.len(), prefix.len() + PHONE.len());
        let (preserved, fallback) = failure(&normalizer, &source, owner);
        let spoken_prefix = if prefix.is_empty() {
            ""
        } else {
            "adres numara beş "
        };
        assert_eq!(
            preserved.normalized_text(),
            format!("{spoken_prefix}{PHONE}{spoken_tail}")
        );
        assert_eq!(
            fallback.normalized_text(),
            format!("{spoken_prefix}{LITERAL}{spoken_tail}")
        );
        let expected = match source_tail {
            " 89" => vec![(1, 3, SegmentKind::Cardinal, "number", "seksen dokuz")],
            " 5 kg" => vec![(1, 5, SegmentKind::Unit, "quantity", "beş kilogram")],
            " 5'ten" => vec![(1, 6, SegmentKind::Cardinal, "number", "beşten")],
            " adres No:5 Kat:3" => vec![
                (7, 10, SegmentKind::Abbreviation, "address.field", "numara "),
                (10, 11, SegmentKind::Cardinal, "number", "beş"),
                (12, 16, SegmentKind::Abbreviation, "address.field", "kat "),
                (16, 17, SegmentKind::Cardinal, "number", "üç"),
            ],
            " Kat:3" => vec![
                (1, 5, SegmentKind::Abbreviation, "address.field", "kat "),
                (5, 6, SegmentKind::Cardinal, "number", "üç"),
            ],
            _ => unreachable!(),
        };
        for result in [&preserved, &fallback] {
            assert_eq!(
                result
                    .segments()
                    .iter()
                    .filter(
                        |s| s.range().start() >= owner.end() && s.kind() != SegmentKind::Verbatim
                    )
                    .map(|s| (
                        s.range().start() - owner.end(),
                        s.range().end() - owner.end(),
                        s.kind(),
                        s.rule_id(),
                        s.text()
                    ))
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn identifier_electronic_and_list_interiors_are_not_telephone_tails() {
    let normalizer = Normalizer::new().unwrap();
    for (tail, category, class) in [
        (
            "AB12",
            IssueCategory::ProtectedIdentifier,
            FallbackClass::Identifier,
        ),
        (
            "67AB12",
            IssueCategory::ProtectedIdentifier,
            FallbackClass::Identifier,
        ),
        (
            "67'den,AB12",
            IssueCategory::InvalidExpression,
            FallbackClass::Number,
        ),
        (
            "67'den,5kg",
            IssueCategory::InvalidExpression,
            FallbackClass::Number,
        ),
        (
            "67'den,Claude",
            IssueCategory::InvalidExpression,
            FallbackClass::Number,
        ),
        (
            "67'den@ornek.com",
            IssueCategory::Unsupported,
            FallbackClass::Electronic,
        ),
    ] {
        let source = format!("0532 123 45 {tail}");
        let result = normalizer.normalize(&source, &Default::default()).unwrap();
        assert_eq!(result.normalized_text(), source);
        assert_eq!(result.issues().len(), 2);
        assert_eq!(result.issues()[0].range(), SourceRange::new(0, 11));
        assert_eq!(
            result.issues()[1].range(),
            SourceRange::new(12, source.len())
        );
        assert_eq!(result.issues()[1].category(), category);
        assert_eq!(
            normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(result.issues().to_vec()))
        );
        let fallback = normalizer
            .normalize(&source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert!(fallback.complete() && fallback.issues().is_empty());
        assert_eq!(fallback.fallbacks().len(), 2);
        assert_eq!(
            fallback.fallbacks()[1].range(),
            SourceRange::new(12, source.len())
        );
        assert_eq!(fallback.fallbacks()[1].attempted_class(), class);
        partition(&source, &result);
        partition(&source, &fallback);
    }
}

#[test]
fn corrected_owner_rejects_component_hints_but_keeps_independent_hints() {
    let normalizer = Normalizer::new().unwrap();
    for policy in POLICIES {
        for (phone, end) in [
            ("0532 123 45-67'den", 8),
            ("0532 123 45(67)'den", 14),
            ("0532 123 45 (67'den)", 19),
            ("0532 123 45 67'd\u{308}en", 11),
        ] {
            for prefix in ["", "O\u{308}nce "] {
                let source = format!("{prefix}{phone}");
                assert_eq!(
                    normalizer.normalize(
                        &source,
                        &NormalizeOptions {
                            ambiguity_policy: policy,
                            hints: vec![Hint::new(
                                SourceRange::new(prefix.len(), prefix.len() + end),
                                HintKind::Digits
                            )],
                        }
                    ),
                    Err(NormalizeError::InvalidHint),
                    "{source} {policy:?}"
                );
            }
        }
        for prefix in ["", "O\u{308}nce "] {
            let source = format!("{prefix}{PHONE}");
            for (start, end, kind) in [
                (0, 11, HintKind::Digits),
                (12, 18, HintKind::Cardinal),
                (0, 18, HintKind::Telephone),
                (0, 18, HintKind::Digits),
            ] {
                assert_eq!(
                    normalizer.normalize(
                        &source,
                        &NormalizeOptions {
                            ambiguity_policy: policy,
                            hints: vec![Hint::new(
                                SourceRange::new(prefix.len() + start, prefix.len() + end),
                                kind
                            )],
                        }
                    ),
                    Err(NormalizeError::InvalidHint)
                );
            }
        }
        let source = format!("{PHONE}; IV");
        let result = normalizer.normalize(
            &source,
            &NormalizeOptions {
                ambiguity_policy: policy,
                hints: vec![Hint::new(
                    SourceRange::new(PHONE.len() + 2, source.len()),
                    HintKind::Roman,
                )],
            },
        );
        if policy == AmbiguityPolicy::Reject {
            assert!(matches!(result, Err(NormalizeError::Unresolved(_))));
        } else {
            let result = result.unwrap();
            assert!(result.normalized_text().ends_with("; dört"));
            assert_eq!(result.segments().last().unwrap().kind(), SegmentKind::Roman);
        }
        let source = "0532 123 45 67 5'ten";
        let result = normalizer
            .normalize(
                source,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    hints: vec![Hint::new(SourceRange::new(0, 14), HintKind::Telephone)],
                },
            )
            .unwrap();
        assert_eq!(result.normalized_text(), format!("{SPOKEN} beşten"));
        assert!(result.complete() && result.fallbacks().is_empty());
    }
}

#[test]
fn owned_closing_parenthesis_keeps_its_grapheme_and_following_identifier_separate() {
    let normalizer = Normalizer::new().unwrap();
    let source = "0532 123 45 (67'den)\u{301}; 5 kg";
    let owner_end = source.find(';').unwrap();
    let (preserved, fallback) = failure_with_strategy(
        &normalizer,
        source,
        SourceRange::new(0, owner_end),
        FallbackStrategy::UnicodeCodePoint,
    );
    assert!(preserved.normalized_text().ends_with("; beş kilogram"));
    assert!(fallback.normalized_text().ends_with("; beş kilogram"));
    let source = "0532 123 45 (67'den)AB12";
    let preserved = normalizer.normalize(source, &Default::default()).unwrap();
    assert_eq!(preserved.issues().len(), 2);
    assert_eq!(preserved.issues()[0].range(), SourceRange::new(0, 20));
    assert_eq!(
        preserved.issues()[1].range(),
        SourceRange::new(20, source.len())
    );
    assert_eq!(
        preserved.issues()[1].category(),
        IssueCategory::ProtectedIdentifier
    );
    partition(source, &preserved);
}

#[test]
fn suffix_after_numeric_parenthesis_does_not_open_quotes_across_another_owner() {
    let normalizer = Normalizer::new().unwrap();
    let first = "0532 123 45(67)'den";
    let second = "0532 123 45 67'";
    let source = format!("{first} ; {second}");
    for policy in POLICIES {
        let result = normalizer.normalize(&source, &options(policy));
        let ranges = if policy == AmbiguityPolicy::Reject {
            let Err(NormalizeError::Unresolved(issues)) = result else {
                panic!("expected two unresolved owners");
            };
            issues.iter().map(|issue| issue.range()).collect::<Vec<_>>()
        } else {
            let result = result.unwrap();
            partition(&source, &result);
            if policy == AmbiguityPolicy::Fallback {
                assert!(result.complete() && result.issues().is_empty());
                assert_eq!(
                    result.normalized_text(),
                    "sıfır beş üç iki bir iki üç dört beş aç parantez altı yedi kapat parantez kesme de e ne ; sıfır beş üç iki bir iki üç dört beş altı yedi kesme"
                );
                result
                    .fallbacks()
                    .iter()
                    .map(|record| record.range())
                    .collect()
            } else {
                assert_eq!(result.normalized_text(), source);
                result.issues().iter().map(|issue| issue.range()).collect()
            }
        };
        assert_eq!(
            ranges,
            vec![
                SourceRange::new(0, first.len()),
                SourceRange::new(first.len() + " ; ".len(), source.len()),
            ]
        );
    }
}

#[test]
fn long_malformed_owners_preserve_work_budgets_and_control_errors() {
    let normalizer = Normalizer::new().unwrap();
    let stem = "0532 123 45 67'";
    for size in [1024, 4096, 8192, 16384, MAX_INPUT_BYTES] {
        let source = format!("{stem}{}", "d".repeat(size - stem.len()));
        let (preserved, fallback) = failure(&normalizer, &source, SourceRange::new(0, size));
        assert_eq!(preserved.normalized_text(), source);
        assert_eq!(fallback.segments().len(), 1);
        assert_eq!(
            fallback.normalized_text(),
            format!(
                "sıfır beş üç iki bir iki üç dört beş altı yedi kesme{}",
                " de".repeat(size - stem.len())
            )
        );
    }
    for count in [1, 64, 256, 512] {
        let source = format!("{PHONE}; ").repeat(count);
        for policy in POLICIES {
            let result = normalizer.normalize(&source, &options(policy));
            if policy == AmbiguityPolicy::Reject {
                let Err(NormalizeError::Unresolved(issues)) = result else {
                    panic!("expected unresolved repeated phones");
                };
                assert_eq!(issues.len(), count);
                continue;
            }
            let result = result.unwrap();
            let ranges: Vec<_> = if policy == AmbiguityPolicy::Fallback {
                assert!(result.complete() && result.issues().is_empty());
                assert_eq!(
                    result.normalized_text(),
                    format!("{LITERAL}; ").repeat(count)
                );
                result.fallbacks().iter().map(|f| f.range()).collect()
            } else {
                assert_eq!(result.normalized_text(), source);
                result.issues().iter().map(|i| i.range()).collect()
            };
            assert_eq!(
                ranges,
                (0..count)
                    .map(|i| {
                        SourceRange::new(i * (PHONE.len() + 2), i * (PHONE.len() + 2) + PHONE.len())
                    })
                    .collect::<Vec<_>>()
            );
            partition(&source, &result);
        }
    }
    let amplified = format!("{stem}{}", "w".repeat(MAX_INPUT_BYTES - stem.len()));
    assert!(matches!(
        normalizer.normalize(&amplified, &options(AmbiguityPolicy::Fallback)),
        Err(NormalizeError::LimitExceeded(LimitKind::Result))
    ));
    for policy in POLICIES {
        for control in [
            {
                let control = WorkControl::default();
                control.cancel();
                control
            },
            WorkControl::new(Some(Instant::now() - Duration::from_secs(1))),
        ] {
            assert_eq!(
                normalizer.normalize_controlled(PHONE, &options(policy), &control),
                Err(NormalizeError::Cancelled)
            );
        }
    }
}
