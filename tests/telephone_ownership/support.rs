//! Shared original-source and policy assertions for telephone acceptance tests.
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, IssueCategory,
    NormalizeError, NormalizeOptions, NormalizeResult, Normalizer, SegmentKind, SourceRange,
};

use unicode_segmentation::UnicodeSegmentation;

pub(super) const PHONE: &str = "0532 123 45 67'den";
pub(super) const LITERAL: &str = "sıfır beş üç iki bir iki üç dört beş altı yedi kesme de e ne";
pub(super) const SPOKEN: &str = "sıfır beş yüz otuz iki yüz yirmi üç kırk beş altmış yedi";
pub(super) const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

pub(super) fn options(policy: AmbiguityPolicy) -> NormalizeOptions {
    NormalizeOptions {
        ambiguity_policy: policy,
        ..Default::default()
    }
}

pub(super) fn partition(input: &str, result: &NormalizeResult) {
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

pub(super) fn failure(
    normalizer: &Normalizer,
    source: &str,
    range: SourceRange,
) -> (NormalizeResult, NormalizeResult) {
    failure_with_strategy(normalizer, source, range, FallbackStrategy::Literal)
}

pub(super) fn failure_with_strategy(
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
