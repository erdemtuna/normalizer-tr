//! Exact source-map workloads and independent public-outcome checks.
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, Hint, HintKind, Issue,
    IssueCategory, MAX_HINTS, NORMALIZER_ID, NormalizeError, NormalizeOptions, NormalizeResult,
    SegmentKind, SourceRange,
};
use unicode_segmentation::UnicodeSegmentation;

use super::support::ScalingCase;

const SIZES: [usize; 5] = [256, 1024, 4096, 16384, 32768];
const CONTROL_BYTES: usize = 1024;
const ASCII_PATTERN: &str = "word  ";
const NFC_PATTERN: &str = "\u{f6} word  ";
const DECOMPOSED_PATTERN: &str = "o\u{308} word  ";
const HINT_PATTERN: &str = "1;\r\n";
const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

fn padded(mut text: String, bytes: usize) -> String {
    assert!(text.len() <= bytes);
    text.extend(std::iter::repeat_n(' ', bytes - text.len()));
    assert_eq!(text.len(), bytes);
    text
}

fn with_policies(cases: &mut Vec<ScalingCase>, id: &'static str, text: String, hints: Vec<Hint>) {
    for policy in POLICIES {
        cases.push(ScalingCase {
            id,
            text: text.clone(),
            options: NormalizeOptions {
                ambiguity_policy: policy,
                hints: hints.clone(),
            },
        });
    }
}

pub(super) fn cases() -> Vec<ScalingCase> {
    let mut cases = Vec::new();
    for bytes in SIZES {
        let count = bytes / ASCII_PATTERN.len();
        for (id, separator) in [
            ("ascii-many-words", "  "),
            ("ascii-crlf-sparse", "  "),
            ("ascii-crlf-dense", "\r\n"),
        ] {
            let mut text = format!("word{separator}").repeat(count);
            if id == "ascii-crlf-sparse" {
                text.replace_range("word".len()..ASCII_PATTERN.len(), "\r\n");
            }
            let text = padded(text, bytes);
            assert_eq!(text.split_whitespace().count(), count);
            let pairs = text
                .as_bytes()
                .windows(2)
                .filter(|pair| *pair == b"\r\n")
                .count();
            assert_eq!(
                pairs,
                match id {
                    "ascii-crlf-sparse" => 1,
                    "ascii-crlf-dense" => count,
                    _ => 0,
                }
            );
            with_policies(&mut cases, id, text, vec![]);
        }
        with_policies(&mut cases, "ascii-single-word", "a".repeat(bytes), vec![]);
        let unicode_count = bytes / DECOMPOSED_PATTERN.len();
        for (id, pattern) in [
            ("unicode-nfc", NFC_PATTERN),
            ("unicode-decomposed", DECOMPOSED_PATTERN),
        ] {
            let text = padded(pattern.repeat(unicode_count), bytes);
            assert_eq!(text.split_whitespace().count(), unicode_count * 2);
            with_policies(&mut cases, id, text, vec![]);
        }
        let text = format!("{}\u{f6}", "a".repeat(bytes - "\u{f6}".len()));
        assert_eq!(text.len(), bytes);
        with_policies(&mut cases, "unicode-nfc-late", text, vec![]);
    }

    let text = HINT_PATTERN.repeat(MAX_HINTS);
    assert_eq!(text.len(), CONTROL_BYTES);
    for (id, count) in [
        ("hints-zero", 0),
        ("hints-one", 1),
        ("hints-255", MAX_HINTS - 1),
        ("hints-256", MAX_HINTS),
        ("hints-reversed-valid", MAX_HINTS),
    ] {
        let mut hints: Vec<_> = (0..count)
            .map(|index| {
                let start = index * HINT_PATTERN.len();
                Hint::new(SourceRange::new(start, start + 1), HintKind::Cardinal)
            })
            .collect();
        if id == "hints-reversed-valid" {
            hints.reverse();
        }
        with_policies(&mut cases, id, text.clone(), hints);
    }
    for (id, prefix, range) in [
        ("hints-invalid-early", "1\r\n1", SourceRange::new(0, 2)),
        (
            "hints-invalid-bounds",
            "1",
            SourceRange::new(0, CONTROL_BYTES + 1),
        ),
        ("hints-invalid-owner", "25 TL", SourceRange::new(0, 2)),
    ] {
        with_policies(
            &mut cases,
            id,
            padded(prefix.to_owned(), CONTROL_BYTES),
            vec![Hint::new(range, HintKind::Cardinal)],
        );
    }
    with_policies(
        &mut cases,
        "policy-unresolved",
        padded("1.234 word".to_owned(), CONTROL_BYTES),
        vec![],
    );
    assert_eq!(cases.len(), 132);
    cases
}

fn check_partition(input: &str, result: &NormalizeResult) {
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
        assert!(boundaries.binary_search(&range.start()).is_ok());
        assert!(boundaries.binary_search(&range.end()).is_ok());
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
    assert_eq!(result.locale(), "tr-TR");
    assert_eq!(result.normalizer_id(), NORMALIZER_ID);
}

fn check_issue(issues: &[Issue]) {
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].range(), SourceRange::new(0, "1.234".len()));
    assert_eq!(issues[0].category(), IssueCategory::Ambiguous);
    assert_eq!(
        issues[0].explanation(),
        "expression requires an explicit supported cue or hint"
    );
}

pub(super) fn check(case: &ScalingCase, outcome: &Result<NormalizeResult, NormalizeError>) {
    match case.id {
        "hints-invalid-early" | "hints-invalid-bounds" | "hints-invalid-owner" => {
            assert_eq!(*outcome, Err(NormalizeError::InvalidHint));
        }
        "policy-unresolved" => {
            if case.options.ambiguity_policy == AmbiguityPolicy::Reject {
                let Err(NormalizeError::Unresolved(issues)) = outcome else {
                    panic!("expected unresolved diagnostics, got {outcome:?}");
                };
                check_issue(issues);
                return;
            }
            let result = outcome.as_ref().unwrap();
            check_partition(&case.text, result);
            let fallback = case.options.ambiguity_policy == AmbiguityPolicy::Fallback;
            assert_eq!(result.complete(), fallback);
            assert_eq!(result.fallback_used(), fallback);
            assert_eq!(result.segments().len(), 2);
            let owner = &result.segments()[0];
            assert_eq!(owner.range(), SourceRange::new(0, "1.234".len()));
            assert_eq!(
                owner.kind(),
                if fallback {
                    SegmentKind::Fallback
                } else {
                    SegmentKind::Unresolved
                }
            );
            assert_eq!(
                owner.rule_id(),
                if fallback {
                    "source.fallback"
                } else {
                    "source.unresolved"
                }
            );
            assert_eq!(
                owner.text(),
                if fallback {
                    "bin iki yüz otuz dört"
                } else {
                    "1.234"
                }
            );
            let tail = &result.segments()[1];
            assert_eq!(
                tail.range(),
                SourceRange::new("1.234".len(), case.text.len())
            );
            assert_eq!(tail.kind(), SegmentKind::Verbatim);
            assert_eq!(tail.rule_id(), "source.verbatim");
            assert_eq!(tail.text(), &case.text["1.234".len()..]);
            assert_eq!(
                result.normalized_text(),
                format!("{}{}", owner.text(), &case.text["1.234".len()..])
            );
            if fallback {
                assert!(result.issues().is_empty());
                assert_eq!(result.fallbacks().len(), 1);
                let record = &result.fallbacks()[0];
                assert_eq!(record.range(), owner.range());
                assert_eq!(record.attempted_class(), FallbackClass::Number);
                assert_eq!(record.reason(), FallbackReason::MissingIntent);
                assert_eq!(record.original_category(), Some(IssueCategory::Ambiguous));
                assert_eq!(record.strategy(), FallbackStrategy::PreferredNumber);
            } else {
                check_issue(result.issues());
                assert!(result.fallbacks().is_empty());
            }
        }
        "hints-zero" | "hints-one" | "hints-255" | "hints-256" | "hints-reversed-valid" => {
            let result = outcome.as_ref().unwrap();
            check_partition(&case.text, result);
            assert!(
                result.complete() && result.issues().is_empty() && result.fallbacks().is_empty()
            );
            assert_eq!(result.normalized_text(), "bir;\r\n".repeat(MAX_HINTS));
            assert_eq!(result.segments().len(), MAX_HINTS * 2);
            for (index, pair) in result.segments().as_chunks::<2>().0.iter().enumerate() {
                let start = index * HINT_PATTERN.len();
                assert_eq!(pair[0].range(), SourceRange::new(start, start + 1));
                assert_eq!(pair[0].kind(), SegmentKind::Cardinal);
                assert_eq!(pair[0].rule_id(), "number");
                assert_eq!(pair[0].text(), "bir");
                assert_eq!(
                    pair[1].range(),
                    SourceRange::new(start + 1, start + HINT_PATTERN.len())
                );
                assert_eq!(pair[1].kind(), SegmentKind::Verbatim);
                assert_eq!(pair[1].rule_id(), "source.verbatim");
                assert_eq!(pair[1].text(), ";\r\n");
            }
        }
        "ascii-single-word" | "ascii-many-words" | "ascii-crlf-sparse" | "ascii-crlf-dense"
        | "unicode-nfc" | "unicode-decomposed" | "unicode-nfc-late" => {
            let result = outcome.as_ref().unwrap();
            check_partition(&case.text, result);
            assert!(
                result.complete() && result.issues().is_empty() && result.fallbacks().is_empty()
            );
            assert_eq!(result.normalized_text(), case.text);
            assert_eq!(result.segments().len(), 1);
            let segment = &result.segments()[0];
            assert_eq!(segment.range(), SourceRange::new(0, case.text.len()));
            assert_eq!(segment.kind(), SegmentKind::Verbatim);
            assert_eq!(segment.rule_id(), "source.verbatim");
        }
        id => panic!("unknown source-map workload {id}"),
    }
}
