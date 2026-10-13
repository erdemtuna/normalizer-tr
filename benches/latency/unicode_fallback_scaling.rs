//! Unicode fallback workloads shared by the unchanged reference and formatter candidate.
use super::support::ScalingCase;
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, LimitKind, MAX_INPUT_BYTES,
    NormalizeError, NormalizeOptions, NormalizeResult, SegmentKind, SourceRange,
};
use unicode_segmentation::UnicodeSegmentation;

const SINGLE: &str = "\u{1FAE0}";
const SINGLE_SPEECH: &str = "unikod u artı bir fe a e sıfır";
const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
const FAMILY_SPEECH: &str = concat!(
    "unikod u artı bir fe dört altı sekiz ",
    "unikod u artı iki sıfır sıfır de ",
    "unikod u artı bir fe dört altı dokuz ",
    "unikod u artı iki sıfır sıfır de ",
    "unikod u artı bir fe dört altı yedi"
);
const VARIATION: &str = "\u{1F642}\u{FE0F}";
const VARIATION_SPEECH: &str = "unikod u artı bir fe altı dört iki unikod u artı fe e sıfır fe";
const NAMED: &str = "\u{1F642}";
const NAMED_SPEECH: &str = "gülümseyen yüz";
const SINGLE_SIZES: &[usize] = &[512, 2048, 8192, 16384];
const CLUSTER_SIZES: &[usize] = &[512, 2048, 8192];
const LIMIT_8000_BYTES: &[usize] = &[32000];
const LIMIT_8192_BYTES: &[usize] = &[32768];
const CASE_COUNT: usize = 48;
const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

pub(super) fn cases() -> Vec<ScalingCase> {
    let mut cases = Vec::with_capacity(CASE_COUNT);
    for (id, pattern, sizes) in [
        ("unnamed-single", SINGLE, SINGLE_SIZES),
        ("unnamed-family", FAMILY, CLUSTER_SIZES),
        ("unnamed-variation", VARIATION, CLUSTER_SIZES),
        ("named-symbol", NAMED, SINGLE_SIZES),
        ("result-limit-8000-scalars", SINGLE, LIMIT_8000_BYTES),
        ("result-limit-8192-scalars", SINGLE, LIMIT_8192_BYTES),
    ] {
        for &bytes in sizes {
            let mut text = pattern.repeat(bytes / pattern.len());
            text.push_str(&" ".repeat(bytes - text.len()));
            assert_eq!(text.len(), bytes);
            assert!(text.len() <= MAX_INPUT_BYTES);
            for ambiguity_policy in POLICIES {
                cases.push(ScalingCase {
                    id,
                    text: text.clone(),
                    options: NormalizeOptions {
                        ambiguity_policy,
                        ..Default::default()
                    },
                });
            }
        }
    }
    assert_eq!(cases.len(), CASE_COUNT);
    cases
}

pub(super) fn check(case: &ScalingCase, outcome: &Result<NormalizeResult, NormalizeError>) {
    let (pattern, speech, sizes, result_limit, strategy) = match case.id {
        "unnamed-single" => (
            SINGLE,
            SINGLE_SPEECH,
            SINGLE_SIZES,
            false,
            FallbackStrategy::UnicodeCodePoint,
        ),
        "unnamed-family" => (
            FAMILY,
            FAMILY_SPEECH,
            CLUSTER_SIZES,
            false,
            FallbackStrategy::UnicodeCodePoint,
        ),
        "unnamed-variation" => (
            VARIATION,
            VARIATION_SPEECH,
            CLUSTER_SIZES,
            false,
            FallbackStrategy::UnicodeCodePoint,
        ),
        "named-symbol" => (
            NAMED,
            NAMED_SPEECH,
            SINGLE_SIZES,
            false,
            FallbackStrategy::Literal,
        ),
        "result-limit-8000-scalars" => (
            SINGLE,
            SINGLE_SPEECH,
            LIMIT_8000_BYTES,
            true,
            FallbackStrategy::UnicodeCodePoint,
        ),
        "result-limit-8192-scalars" => (
            SINGLE,
            SINGLE_SPEECH,
            LIMIT_8192_BYTES,
            true,
            FallbackStrategy::UnicodeCodePoint,
        ),
        id => panic!("unknown Unicode fallback workload: {id}"),
    };
    assert!(case.options.hints.is_empty());
    assert!(sizes.contains(&case.text.len()), "{}", case.id);
    assert!(case.text.len() <= MAX_INPUT_BYTES);
    let count = case.text.len() / pattern.len();
    let run_bytes = count * pattern.len();
    let source_run = pattern.repeat(count);
    assert_eq!(&case.text[..run_bytes], source_run, "{}", case.id);
    let padding = &case.text[run_bytes..];
    assert_eq!(padding, " ".repeat(case.text.len() - run_bytes));
    if case.options.ambiguity_policy == AmbiguityPolicy::Fallback && result_limit {
        assert_eq!(
            *outcome,
            Err(NormalizeError::LimitExceeded(LimitKind::Result)),
            "{}",
            case.id
        );
        return;
    }
    let result = outcome.as_ref().unwrap();
    assert!(result.complete());
    assert!(result.issues().is_empty());
    assert_eq!(result.locale(), "tr-TR");
    if case.options.ambiguity_policy == AmbiguityPolicy::Fallback {
        let expected = format!("{}{padding}", vec![speech; count].join(" "));
        assert_eq!(result.normalized_text(), expected, "{}", case.id);
        assert!(result.fallback_used());
        assert_eq!(result.fallbacks().len(), 1);
        let owner = SourceRange::new(0, run_bytes);
        let record = &result.fallbacks()[0];
        assert_eq!(record.range(), owner);
        assert_eq!(record.attempted_class(), FallbackClass::Symbol);
        assert_eq!(record.reason(), FallbackReason::UnhandledSymbol);
        assert_eq!(record.original_category(), None);
        assert_eq!(record.strategy(), strategy);
        assert_eq!(
            result.segments().len(),
            1 + usize::from(!padding.is_empty())
        );
        let segment = &result.segments()[0];
        assert_eq!(segment.range(), owner);
        assert_eq!(segment.kind(), SegmentKind::Fallback);
        assert_eq!(segment.rule_id(), "source.fallback");
        if !padding.is_empty() {
            let segment = &result.segments()[1];
            assert_eq!(
                segment.range(),
                SourceRange::new(run_bytes, case.text.len())
            );
            assert_eq!(segment.kind(), SegmentKind::Verbatim);
            assert_eq!(segment.text(), padding);
            assert_eq!(segment.rule_id(), "source.verbatim");
        }
    } else {
        assert_eq!(result.normalized_text(), case.text);
        assert!(!result.fallback_used());
        assert!(result.fallbacks().is_empty());
        assert_eq!(result.segments().len(), 1);
        let segment = &result.segments()[0];
        assert_eq!(segment.range(), SourceRange::new(0, case.text.len()));
        assert_eq!(segment.kind(), SegmentKind::Verbatim);
        assert_eq!(segment.rule_id(), "source.verbatim");
    }
    let mut boundaries: Vec<_> = case
        .text
        .grapheme_indices(true)
        .map(|(start, _)| start)
        .collect();
    boundaries.push(case.text.len());
    let mut cursor = 0;
    for segment in result.segments() {
        assert_eq!(segment.range().start(), cursor);
        cursor = segment.range().end();
        assert!(boundaries.contains(&cursor));
        assert!(!segment.text().is_empty());
    }
    assert_eq!(cursor, case.text.len());
    assert_eq!(
        result
            .segments()
            .iter()
            .map(|segment| segment.text())
            .collect::<String>(),
        result.normalized_text()
    );
}
