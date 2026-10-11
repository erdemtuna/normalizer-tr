//! Generated compositions around the reader ownership failures found in review.
use std::sync::OnceLock;

use normalizer_tr::{
    AmbiguityPolicy, NormalizeError, NormalizeOptions, NormalizeResult, Normalizer, SourceRange,
};
use proptest::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

const SEPARATOR: &str = " ; ";
const MIXED: [&str; 5] = ["2 x 3 +4", "2*3=6", "2 x 3 / 4", "2*3 + 4", "2 x 3 - 4"];
const INDEPENDENT: [&str; 4] = [
    "2.000-3.000 kişi",
    "1.234 - 2.345 kg",
    "tarih 31.12.2026 - toplantı",
    "adres No:5 2 x 3 Kat:3",
];

fn normalizer() -> &'static Normalizer {
    static NORMALIZER: OnceLock<Normalizer> = OnceLock::new();
    NORMALIZER.get_or_init(|| Normalizer::new().unwrap())
}

fn partition(source: &str, result: &NormalizeResult) {
    let mut cursor = 0;
    let mut boundaries: Vec<_> = source.grapheme_indices(true).map(|(i, _)| i).collect();
    boundaries.push(source.len());
    for segment in result.segments() {
        assert_eq!(segment.range().start(), cursor);
        cursor = segment.range().end();
        assert!(boundaries.contains(&cursor));
    }
    assert_eq!(cursor, source.len());
    assert_eq!(
        result
            .segments()
            .iter()
            .map(|s| s.text())
            .collect::<String>(),
        result.normalized_text()
    );
}

fn owner(result: &NormalizeResult, expected: SourceRange) {
    assert_eq!(result.issues().len(), 1);
    assert_eq!(result.issues()[0].range(), expected);
}

proptest! {
    #[test]
    fn failed_expression_does_not_capture_later_reader_fields(
        mixed in prop::sample::select(&MIXED[..]),
        fields in prop::collection::vec(prop::sample::select(&INDEPENDENT[..]), 1..5),
        tail in 0..256_usize,
        prefix in prop::sample::select(&["ö", "o\u{308}", "İstanbul"][..]),
    ) {
        let n = normalizer();
        let url = format!("https://ornek.com/{}", ")".repeat(tail));
        let mut parts = vec![prefix, mixed];
        parts.extend(fields);
        parts.push(url.as_str());
        let source = parts.join(SEPARATOR);
        let expected_range = SourceRange::new(prefix.len() + SEPARATOR.len(), prefix.len() + SEPARATOR.len() + mixed.len());
        let preserved = n.normalize(&source, &Default::default()).unwrap();
        owner(&preserved, expected_range);
        partition(&source, &preserved);
        prop_assert_eq!(
            n.normalize(&source, &NormalizeOptions { ambiguity_policy: AmbiguityPolicy::Reject, ..Default::default() }),
            Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
        );
        for policy in [AmbiguityPolicy::Preserve, AmbiguityPolicy::Fallback] {
            let options = NormalizeOptions { ambiguity_policy: policy, ..Default::default() };
            let result = n.normalize(&source, &options).unwrap();
            let expected = parts.iter().map(|part| n.normalize(part, &options).unwrap().normalized_text().to_owned()).collect::<Vec<_>>().join(SEPARATOR);
            prop_assert_eq!(result.normalized_text(), expected);
            partition(&source, &result);
            prop_assert_eq!(n.normalize(&source, &options).unwrap(), result.clone());
            if policy == AmbiguityPolicy::Fallback {
                prop_assert!(result.complete());
                prop_assert_eq!(result.fallbacks().len(), 1);
                prop_assert_eq!(result.fallbacks()[0].range(), expected_range);
            }
        }
    }
}
