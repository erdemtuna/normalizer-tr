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
            "phone-suffixed-marked-closer-before-tail",
            "phone-suffixed-marked-closer-after-tail",
            "phone-suffixed-repeated-after-closer",
            "phone-suffixed-empty-after-closer",
            "phone-suffixed-digit-after-closer",
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
            ("0532 123 45(67)\u{301}'den", 14),
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
fn marked_closer_does_not_skip_protected_or_primary_neighbors() {
    let normalizer = Normalizer::new().unwrap();
    let phone = "0532 123 45 (67'den)\u{301}";
    for prefix in ["", "O\u{308}nce "] {
        for (neighbor, kind, rule, spoken) in [
            ("AB12", SegmentKind::Unresolved, "source.unresolved", "AB12"),
            ("5kg", SegmentKind::Unit, "quantity", "beş kilogram"),
            (
                "Claude",
                SegmentKind::Pronunciation,
                "pronunciation.name",
                "klod",
            ),
            ("12", SegmentKind::Cardinal, "number", "on iki"),
        ] {
            let source = format!("{prefix}{phone}{neighbor}; 5 kg");
            let phone_range = SourceRange::new(prefix.len(), prefix.len() + phone.len());
            let neighbor_range =
                SourceRange::new(phone_range.end(), phone_range.end() + neighbor.len());
            let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
            partition(&source, &preserved);
            assert_eq!(preserved.issues()[0].range(), phone_range);
            let segment = preserved
                .segments()
                .iter()
                .find(|s| s.range() == neighbor_range)
                .unwrap();
            assert_eq!(
                (segment.kind(), segment.rule_id(), segment.text()),
                (kind, rule, spoken)
            );
            assert_eq!(
                preserved.issues().len(),
                if neighbor == "AB12" { 2 } else { 1 }
            );
            if neighbor == "AB12" {
                assert_eq!(preserved.issues()[1].range(), neighbor_range);
                assert_eq!(
                    preserved.issues()[1].category(),
                    IssueCategory::ProtectedIdentifier
                );
            }

            assert_eq!(
                normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
                Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
            );
            let fallback = normalizer
                .normalize(&source, &options(AmbiguityPolicy::Fallback))
                .unwrap();
            partition(&source, &fallback);
            assert!(fallback.complete() && fallback.issues().is_empty());
            assert_eq!(fallback.fallbacks()[0].range(), phone_range);
            assert_eq!(
                fallback.fallbacks()[0].strategy(),
                FallbackStrategy::UnicodeCodePoint
            );
            if neighbor == "AB12" {
                assert_eq!(fallback.fallbacks().len(), 2);
                assert_eq!(fallback.fallbacks()[1].range(), neighbor_range);
                assert_eq!(
                    fallback.fallbacks()[1].reason(),
                    FallbackReason::ProtectedIdentifier
                );
                assert_eq!(
                    fallback.fallbacks()[1].attempted_class(),
                    FallbackClass::Identifier
                );
            } else {
                assert_eq!(fallback.fallbacks().len(), 1);
                assert!(
                    fallback
                        .segments()
                        .iter()
                        .any(|s| s.range() == neighbor_range
                            && s.kind() == kind
                            && s.text() == spoken)
                );
            }
        }
    }
}

#[test]
fn maximal_marked_closer_remains_bounded_before_address_fields() {
    let normalizer = Normalizer::new().unwrap();
    for count in [64, 16000] {
        let source = format!(
            "adres No:5 0532 123 45 (67'den){} Kat:3",
            "\u{308}".repeat(count)
        );
        assert!(source.len() <= MAX_INPUT_BYTES);
        let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
        partition(&source, &preserved);
        let owner = SourceRange::new("adres No:5 ".len(), source.find(" Kat:3").unwrap());
        assert_eq!(preserved.issues().len(), 1);
        assert_eq!(preserved.issues()[0].range(), owner);
        assert!(preserved.normalized_text().ends_with(" kat üç"));
        assert_eq!(
            normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
        );
        match normalizer.normalize(&source, &options(AmbiguityPolicy::Fallback)) {
            Ok(result) => {
                partition(&source, &result);
                assert!(result.complete() && result.issues().is_empty());
                assert_eq!(result.fallbacks().len(), 1);
                assert_eq!(result.fallbacks()[0].range(), owner);
                assert!(result.normalized_text().ends_with(" kat üç"));
            }
            Err(NormalizeError::LimitExceeded(LimitKind::Result)) => assert_eq!(count, 16000),
            other => panic!("unexpected bounded marked-closer outcome: {other:?}"),
        }
    }
}

#[test]
fn a_marked_closing_delimiter_does_not_glue_the_following_number() {
    let result = Normalizer::new()
        .unwrap()
        .normalize(")\u{301}12", &Default::default())
        .unwrap();
    assert!(result.complete() && result.issues().is_empty());
    assert_eq!(result.normalized_text(), ")\u{301}on iki");
    assert_eq!(result.segments()[0].range(), SourceRange::new(0, 3));
    assert_eq!(result.segments()[0].kind(), SegmentKind::Verbatim);
    assert_eq!(result.segments()[1].range(), SourceRange::new(3, 5));
    assert_eq!(result.segments()[1].kind(), SegmentKind::Cardinal);
    partition(")\u{301}12", &result);
}

#[test]
fn repeated_post_closer_tails_cannot_quote_away_a_later_phone_failure() {
    let normalizer = Normalizer::new().unwrap();
    let second = "0532 123 45 67'";
    for (tail, literal) in [
        ("'de", "kesme de e"),
        ("'d2", "kesme de iki"),
        ("'den'de", "kesme de e ne kesme de e"),
        ("'", "kesme"),
    ] {
        let first = format!("0532 123 45 (67'den){tail}");
        for prefix in ["", "O\u{308}nce "] {
            let source = format!("{prefix}{first} ; {second}; 5 kg");
            let first_range = SourceRange::new(prefix.len(), prefix.len() + first.len());
            let second_start = first_range.end() + " ; ".len();
            let second_range = SourceRange::new(second_start, second_start + second.len());
            let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
            assert_eq!(
                preserved.normalized_text(),
                format!("{prefix}{first} ; {second}; beş kilogram")
            );
            assert_eq!(
                preserved
                    .issues()
                    .iter()
                    .map(|i| i.range())
                    .collect::<Vec<_>>(),
                [first_range, second_range]
            );
            assert!(
                preserved
                    .segments()
                    .iter()
                    .all(|s| s.kind() != SegmentKind::Telephone)
            );
            partition(&source, &preserved);
            assert_eq!(
                normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
                Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
            );
            let fallback = normalizer
                .normalize(&source, &options(AmbiguityPolicy::Fallback))
                .unwrap();
            assert!(fallback.complete() && fallback.issues().is_empty());
            assert_eq!(
                fallback
                    .fallbacks()
                    .iter()
                    .map(|f| f.range())
                    .collect::<Vec<_>>(),
                [first_range, second_range]
            );
            assert_eq!(
                fallback.normalized_text(),
                format!(
                    "{prefix}sıfır beş üç iki bir iki üç dört beş aç parantez altı yedi kesme de e ne kapat parantez {literal} ; sıfır beş üç iki bir iki üç dört beş altı yedi kesme; beş kilogram"
                )
            );
            partition(&source, &fallback);
            for policy in POLICIES {
                assert_eq!(
                    normalizer.normalize(
                        &source,
                        &NormalizeOptions {
                            ambiguity_policy: policy,
                            hints: vec![Hint::new(
                                SourceRange::new(second_start, second_range.end() - 1),
                                HintKind::Telephone
                            )],
                        }
                    ),
                    Err(NormalizeError::InvalidHint)
                );
            }
        }
    }
}

#[test]
fn empty_parenthesis_tails_keep_external_separators_and_neighbors_independent() {
    let normalizer = Normalizer::new().unwrap();
    let phone = "0532 123 45(67')'";
    for (separator, neighbor, spoken) in [
        ("; ", "5 kg", "beş kilogram"),
        (";", "5kg", "beş kilogram"),
        (";", "5", "beş"),
    ] {
        let source = format!("{phone}{separator}{neighbor}");
        let (preserved, fallback) = failure(&normalizer, &source, SourceRange::new(0, phone.len()));
        assert_eq!(
            preserved.normalized_text(),
            format!("{phone}{separator}{spoken}")
        );
        assert!(
            fallback
                .normalized_text()
                .ends_with(&format!("{separator}{spoken}"))
        );
        assert_eq!(
            preserved.segments().last().unwrap().range(),
            SourceRange::new(phone.len() + separator.len(), source.len())
        );
    }
    let source = "2(3');5";
    let preserved = normalizer.normalize(source, &Default::default()).unwrap();
    assert_eq!(preserved.normalized_text(), "2(3');beş");
    assert_eq!(preserved.issues().len(), 1);
    assert_eq!(preserved.issues()[0].range(), SourceRange::new(0, 5));
    partition(source, &preserved);
}

#[test]
fn alternating_closers_and_tails_remain_one_bounded_owner() {
    let normalizer = Normalizer::new().unwrap();
    for count in [1, 16, 128, 512] {
        let source = format!(
            "0532 123 {}67'den{}",
            "(".repeat(count),
            ")'de".repeat(count)
        );
        let (preserved, fallback) =
            failure(&normalizer, &source, SourceRange::new(0, source.len()));
        assert_eq!(preserved.normalized_text(), source);
        assert_eq!(fallback.fallbacks().len(), 1);
    }
}

#[test]
fn empty_tail_compounds_preserve_confirmed_wrappers_not_invalid_local_closers() {
    let normalizer = Normalizer::new().unwrap();
    let phone = "0532 123 45(67')'";
    let literal =
        "sıfır beş üç iki bir iki üç dört beş aç parantez altı yedi kesme kapat parantez kesme";
    for (prefix, closing) in [
        ("(", ")"),
        ("((", "))"),
        ("O\u{308}nce (", ")"),
        ("‘", "’"),
        ("\"", "\""),
    ] {
        for tail in ["; 5 kg", ";5kg", ";AB12"] {
            let source = format!("{prefix}{phone}{closing}{tail}");
            let owner = SourceRange::new(prefix.len(), prefix.len() + phone.len());
            let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
            assert_eq!(preserved.issues()[0].range(), owner, "{source}");
            partition(&source, &preserved);
            assert_eq!(
                normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
                Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
            );
            let fallback = normalizer
                .normalize(&source, &options(AmbiguityPolicy::Fallback))
                .unwrap();
            assert_eq!(fallback.fallbacks()[0].range(), owner);
            let phone_segment = fallback
                .segments()
                .iter()
                .find(|s| s.range() == owner)
                .unwrap();
            assert_eq!(phone_segment.text(), literal);
            assert!(
                fallback
                    .normalized_text()
                    .starts_with(&format!("{prefix}{literal}{closing}"))
            );
            partition(&source, &fallback);
        }
    }
    for source in ["0532 123 45(67')')", "0532 (123 45(67')')'"] {
        let (preserved, _) = failure(&normalizer, source, SourceRange::new(0, source.len()));
        assert_eq!(preserved.normalized_text(), source);
    }
    for (prefix, closing) in [
        ("AB12(foo ; ", ""),
        ("'(' ; ", ""),
        ("\"(\" ; ", ""),
        ("“(” ; ", ""),
        ("‘\"(\" ; ", "’"),
        ("\"'(' ; ", "\""),
    ] {
        let source = format!("{prefix}{phone}){closing}");
        let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
        assert_eq!(
            preserved.issues().last().unwrap().range(),
            SourceRange::new(prefix.len(), source.len() - closing.len())
        );
        partition(&source, &preserved);
    }
}

#[test]
fn maximal_alternating_tail_chain_keeps_its_owner_or_reports_a_real_result_limit() {
    let normalizer = Normalizer::new().unwrap();
    let prefix = "0532 123 ";
    let middle = "67'den";
    let opening = "(";
    let closing_tail = ")'de";
    let count =
        (MAX_INPUT_BYTES - prefix.len() - middle.len()) / (opening.len() + closing_tail.len());
    let source = format!(
        "{prefix}{}{middle}{}",
        opening.repeat(count),
        closing_tail.repeat(count)
    );
    let owner = SourceRange::new(0, source.len());
    let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
    assert_eq!(preserved.normalized_text(), source);
    assert_eq!(preserved.issues().len(), 1);
    assert_eq!(preserved.issues()[0].range(), owner);
    assert_eq!(
        normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
        Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
    );
    match normalizer.normalize(&source, &options(AmbiguityPolicy::Fallback)) {
        Ok(result) => {
            partition(&source, &result);
            assert!(result.complete() && result.issues().is_empty());
            assert_eq!(result.fallbacks().len(), 1);
            assert_eq!(result.fallbacks()[0].range(), owner);
        }
        Err(NormalizeError::LimitExceeded(LimitKind::Result)) => {}
        other => panic!("unexpected maximal tail-chain outcome: {other:?}"),
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
