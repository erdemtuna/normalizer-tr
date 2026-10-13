//! Original UTF-8 grapheme mapping and hint validity.
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, Hint, HintKind,
    IssueCategory, NormalizeError, NormalizeOptions, NormalizeResult, Normalizer, SegmentKind,
    SourceRange,
};
use unicode_segmentation::UnicodeSegmentation;

const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

fn assert_partition(input: &str, result: &NormalizeResult) {
    let boundaries: Vec<_> = input
        .grapheme_indices(true)
        .map(|(start, _)| start)
        .chain(std::iter::once(input.len()))
        .collect();
    let mut cursor = 0;
    for segment in result.segments() {
        let range = segment.range();
        assert_eq!(range.start(), cursor);
        assert!(range.start() < range.end());
        assert!(boundaries.contains(&range.start()));
        assert!(boundaries.contains(&range.end()));
        if matches!(
            segment.kind(),
            SegmentKind::Verbatim | SegmentKind::Unresolved
        ) {
            assert_eq!(segment.text(), &input[range.start()..range.end()]);
        }
        cursor = range.end();
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
    assert_eq!(result.complete(), result.issues().is_empty());
    assert_eq!(result.fallback_used(), !result.fallbacks().is_empty());
    for range in result
        .issues()
        .iter()
        .map(|issue| issue.range())
        .chain(result.fallbacks().iter().map(|record| record.range()))
    {
        assert!(range.start() < range.end());
        assert!(boundaries.contains(&range.start()));
        assert!(boundaries.contains(&range.end()));
    }
}

#[test]
fn nfc_recognition_keeps_original_graphemes_and_case() {
    let input = "İstanbul 👩‍👩‍👧‍👦: o\u{308}n 4’u\u{308}n ve 25 TL";
    let result = Normalizer::new()
        .unwrap()
        .normalize(input, &NormalizeOptions::default())
        .unwrap();
    assert_eq!(
        result.normalized_text(),
        "İstanbul 👩‍👩‍👧‍👦: o\u{308}n dördün ve yirmi beş Türk lirası"
    );
    assert!(result.complete());
    let inflected = result
        .segments()
        .iter()
        .find(|s| s.kind() == SegmentKind::Cardinal)
        .unwrap();
    assert_eq!(
        &input[inflected.range().start()..inflected.range().end()],
        "4’u\u{308}n"
    );
    assert_eq!(
        result
            .segments()
            .iter()
            .map(|s| s.text())
            .collect::<String>(),
        result.normalized_text()
    );
}

#[test]
fn hint_ranges_are_original_non_overlapping_grapheme_safe_whole_expressions() {
    let normalizer = Normalizer::new().unwrap();
    let input = "e\u{301} 00042";
    let start = input.find('0').unwrap();
    let result = normalizer
        .normalize(
            input,
            &NormalizeOptions {
                hints: vec![Hint::new(
                    SourceRange::new(start, input.len()),
                    HintKind::Digits,
                )],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        result.normalized_text(),
        "e\u{301} sıfır sıfır sıfır dört iki"
    );
    for (input, hints) in [
        (
            "e\u{301} 12",
            vec![Hint::new(SourceRange::new(0, 1), HintKind::Digits)],
        ),
        (
            "ü 12",
            vec![Hint::new(SourceRange::new(1, 2), HintKind::Digits)],
        ),
        (
            "12",
            vec![Hint::new(SourceRange::new(0, 3), HintKind::Cardinal)],
        ),
        (
            "12",
            vec![Hint::new(SourceRange::new(1, 1), HintKind::Cardinal)],
        ),
        (
            "123",
            vec![Hint::new(SourceRange::new(0, 2), HintKind::Cardinal)],
        ),
        (
            "AB123",
            vec![Hint::new(SourceRange::new(2, 5), HintKind::Cardinal)],
        ),
        (
            "25 TL",
            vec![Hint::new(SourceRange::new(0, 2), HintKind::Cardinal)],
        ),
        (
            "12",
            vec![Hint::new(SourceRange::new(0, 2), HintKind::Cardinal); 2],
        ),
        (
            "00,42",
            vec![Hint::new(SourceRange::new(0, 5), HintKind::Cardinal)],
        ),
        (
            "31.04.2026",
            vec![Hint::new(SourceRange::new(0, 10), HintKind::Date)],
        ),
        (
            "24:00",
            vec![Hint::new(SourceRange::new(0, 5), HintKind::Time)],
        ),
        (
            "+90 532 123",
            vec![Hint::new(SourceRange::new(0, 3), HintKind::Digits)],
        ),
    ] {
        assert_eq!(
            normalizer.normalize(
                input,
                &NormalizeOptions {
                    hints,
                    ..Default::default()
                }
            ),
            Err(NormalizeError::InvalidHint),
            "{input}"
        );
    }
}

#[test]
fn punctuation_with_combining_marks_remains_grapheme_safe() {
    let normalizer = Normalizer::new().unwrap();
    for input in [";\u{301}12", ",\u{301}12"] {
        let result = normalizer
            .normalize(input, &NormalizeOptions::default())
            .unwrap();
        assert_eq!(result.normalized_text(), input);
        assert!(!result.complete());
        assert_eq!(result.issues()[0].range(), SourceRange::new(0, input.len()));
    }
    for input in ["e;\u{301}12", "12;\u{301}34", "\u{301} 25 TL"] {
        let result = normalizer
            .normalize(input, &NormalizeOptions::default())
            .unwrap();
        assert_eq!(
            result
                .segments()
                .iter()
                .map(|s| s.text())
                .collect::<String>(),
            result.normalized_text()
        );
        for segment in result.segments() {
            assert!(input.is_char_boundary(segment.range().start()));
            assert!(input.is_char_boundary(segment.range().end()));
        }
    }
}

#[test]
fn curly_abbreviation_lookup_preserves_the_original_whole_range() {
    let input = "ü TBMM’ye!";
    let result = Normalizer::new()
        .unwrap()
        .normalize(input, &NormalizeOptions::default())
        .unwrap();
    assert_eq!(result.normalized_text(), "ü te be me meye!");
    assert!(result.complete());
    let abbreviation = result
        .segments()
        .iter()
        .find(|s| s.kind() == SegmentKind::Abbreviation)
        .unwrap();
    assert_eq!(
        &input[abbreviation.range().start()..abbreviation.range().end()],
        "TBMM’ye"
    );
}

#[test]
fn maximum_hints_are_accepted_and_sorted_without_changing_gaps() {
    let input = "1;".repeat(normalizer_tr::MAX_HINTS);
    let hints = (0..normalizer_tr::MAX_HINTS)
        .rev()
        .map(|index| {
            Hint::new(
                SourceRange::new(index * 2, index * 2 + 1),
                HintKind::Cardinal,
            )
        })
        .collect();
    let result = Normalizer::new()
        .unwrap()
        .normalize(
            &input,
            &NormalizeOptions {
                hints,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(result.complete());
    assert_eq!(
        result.normalized_text(),
        "bir;".repeat(normalizer_tr::MAX_HINTS)
    );
}

#[test]
fn digits_hints_consume_all_promised_separators_at_original_ranges() {
    let normalizer = Normalizer::new().unwrap();
    for expression in ["1.234", "12/34", "(123)", "+90 (532) 123 45 67"] {
        let input = format!("ö\u{308} {expression} sonra");
        let start = "ö\u{308} ".len();
        let range = SourceRange::new(start, start + expression.len());
        let result = normalizer
            .normalize(
                &input,
                &NormalizeOptions {
                    hints: vec![Hint::new(range, HintKind::Digits)],
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(result.complete());
        let digits = result
            .segments()
            .iter()
            .find(|s| s.kind() == SegmentKind::Digits)
            .unwrap();
        assert_eq!(digits.range(), range);
        assert_eq!(&input[range.start()..range.end()], expression);
    }
    for (input, range) in [
        ("12/34", SourceRange::new(0, 2)),
        ("+90 (532) 123 45 67", SourceRange::new(5, 8)),
        ("12@34", SourceRange::new(0, 5)),
        ("12:34", SourceRange::new(0, 5)),
        ("12\t34", SourceRange::new(0, 5)),
        ("12+34", SourceRange::new(0, 5)),
    ] {
        assert_eq!(
            normalizer.normalize(
                input,
                &NormalizeOptions {
                    hints: vec![Hint::new(range, HintKind::Digits)],
                    ..Default::default()
                }
            ),
            Err(NormalizeError::InvalidHint)
        );
    }
}

#[test]
fn colon_cues_use_nfc_lookup_but_keep_original_case_and_coordinates() {
    let input = "TARI\u{307}H:01.02.2026 ve saat:09:30";
    let result = Normalizer::new()
        .unwrap()
        .normalize(input, &NormalizeOptions::default())
        .unwrap();
    assert_eq!(
        result.normalized_text(),
        "TARI\u{307}H:bir Şubat iki bin yirmi altı ve saat:dokuz otuz"
    );
    assert!(result.complete());
    for (kind, expression) in [
        (SegmentKind::Date, "01.02.2026"),
        (SegmentKind::Time, "09:30"),
    ] {
        let start = input.find(expression).unwrap();
        let segment = result.segments().iter().find(|s| s.kind() == kind).unwrap();
        assert_eq!(
            segment.range(),
            SourceRange::new(start, start + expression.len())
        );
    }
}

#[test]
fn allowed_ascii_controls_preserve_graphemes_in_all_policies() {
    let normalizer = Normalizer::new().unwrap();
    for input in [
        "word\tword\rword\nword\r\nword",
        "\r\nword\r\n",
        "word\r\r\nword\n",
        "word\r\n\nword",
    ] {
        for policy in POLICIES {
            let result = normalizer
                .normalize(
                    input,
                    &NormalizeOptions {
                        ambiguity_policy: policy,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(result.normalized_text(), input);
            assert!(result.complete());
            assert!(result.issues().is_empty() && result.fallbacks().is_empty());
            assert_partition(input, &result);
            for segment in result.segments() {
                assert_eq!(segment.kind(), SegmentKind::Verbatim);
                assert_eq!(segment.rule_id(), "source.verbatim");
            }
        }
    }
}

#[test]
fn original_hints_do_not_split_crlf_or_weaken_whole_owner_validation() {
    let normalizer = Normalizer::new().unwrap();
    let hint = |start, end| Hint::new(SourceRange::new(start, end), HintKind::Cardinal);
    for policy in POLICIES {
        for (input, hints) in [
            ("1\r\n2", vec![hint(0, 2)]),
            ("1\r\n2", vec![hint(2, 4)]),
            ("1\r\n2", vec![hint(1, 2)]),
            ("1\r\n2", vec![hint(2, 3)]),
            ("1\r\n2", vec![hint(0, 5)]),
            ("1\r\n2", vec![hint(0, usize::MAX)]),
            ("1\r\n2", vec![hint(usize::MAX, usize::MAX)]),
            ("1\r\n2", vec![hint(3, 3)]),
            ("1\r\n2", vec![hint(4, 3)]),
            ("1\r\n2", vec![hint(0, 4), hint(0, 1)]),
            ("1\r\n2", vec![hint(0, 1), hint(0, 1)]),
            ("12", vec![hint(0, 1), hint(1, 2)]),
            ("25 TL\r\n2", vec![hint(0, 2)]),
            ("AB12\r\n2", vec![hint(2, 4)]),
            ("e\u{301}\r\n2", vec![hint(0, 1)]),
            ("ü\r\n2", vec![hint(1, 2)]),
        ] {
            assert_eq!(
                normalizer.normalize(
                    input,
                    &NormalizeOptions {
                        ambiguity_policy: policy,
                        hints,
                    },
                ),
                Err(NormalizeError::InvalidHint),
                "{input:?} {policy:?}"
            );
        }
        let input = "1;\r\n2;\t3";
        let result = normalizer
            .normalize(
                input,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    hints: vec![hint(7, 8), hint(0, 1), hint(4, 5)],
                },
            )
            .unwrap();
        assert_eq!(result.normalized_text(), "bir;\r\niki;\tüç");
        assert!(result.complete());
        assert!(result.issues().is_empty() && result.fallbacks().is_empty());
        assert_partition(input, &result);
        assert_eq!(
            result
                .segments()
                .iter()
                .filter(|s| s.kind() == SegmentKind::Cardinal)
                .map(|s| s.range())
                .collect::<Vec<_>>(),
            [
                SourceRange::new(0, 1),
                SourceRange::new(4, 5),
                SourceRange::new(7, 8)
            ]
        );
    }
}

#[test]
fn decomposed_prefix_and_crlf_keep_original_policy_diagnostics() {
    let normalizer = Normalizer::new().unwrap();
    let prefix = "o\u{308}\r\n";
    let input = format!("{prefix}1.234; word");
    let range = SourceRange::new(prefix.len(), prefix.len() + "1.234".len());
    let preserved = normalizer
        .normalize(&input, &NormalizeOptions::default())
        .unwrap();
    assert_eq!(preserved.normalized_text(), input);
    assert!(!preserved.complete());
    assert_partition(&input, &preserved);
    assert_eq!(preserved.issues().len(), 1);
    assert_eq!(preserved.issues()[0].range(), range);
    assert_eq!(preserved.issues()[0].category(), IssueCategory::Ambiguous);
    assert_eq!(
        normalizer.normalize(
            &input,
            &NormalizeOptions {
                ambiguity_policy: AmbiguityPolicy::Reject,
                ..Default::default()
            },
        ),
        Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
    );
    let fallback = normalizer
        .normalize(
            &input,
            &NormalizeOptions {
                ambiguity_policy: AmbiguityPolicy::Fallback,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        fallback.normalized_text(),
        format!("{prefix}bin iki yüz otuz dört; word")
    );
    assert!(fallback.complete() && fallback.issues().is_empty());
    assert_partition(&input, &fallback);
    assert_eq!(fallback.fallbacks().len(), 1);
    let record = &fallback.fallbacks()[0];
    assert_eq!(record.range(), range);
    assert_eq!(record.attempted_class(), FallbackClass::Number);
    assert_eq!(record.reason(), FallbackReason::MissingIntent);
    assert_eq!(record.original_category(), Some(IssueCategory::Ambiguous));
    assert_eq!(record.strategy(), FallbackStrategy::PreferredNumber);
}
