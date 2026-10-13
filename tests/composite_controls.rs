//! Composite work limits and projection to original Unicode source.
use normalizer_tr::{
    AmbiguityPolicy, LimitKind, NormalizeError, NormalizeOptions, Normalizer, SegmentKind,
    WorkControl,
};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn product_components_do_not_evade_work_limits_or_cancellation() {
    let n = Normalizer::new().unwrap();
    let source = format!("{}1", "1*".repeat(2048));
    for policy in [
        AmbiguityPolicy::Preserve,
        AmbiguityPolicy::Reject,
        AmbiguityPolicy::Fallback,
    ] {
        let options = NormalizeOptions {
            ambiguity_policy: policy,
            ..Default::default()
        };
        assert_eq!(
            n.normalize(&source, &options),
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates)),
        );
        let control = WorkControl::default();
        control.cancel();
        assert_eq!(
            n.normalize_controlled(&source, &options, &control),
            Err(NormalizeError::Cancelled)
        );
    }
}

#[test]
fn constituent_ranges_project_to_original_decomposed_graphemes() {
    let normalizer = Normalizer::new().unwrap();
    let source = "ö e\u{301}\r\n12*833,25TL";
    let boundaries: Vec<_> = source
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(std::iter::once(source.len()))
        .collect();
    for policy in [
        AmbiguityPolicy::Preserve,
        AmbiguityPolicy::Reject,
        AmbiguityPolicy::Fallback,
    ] {
        let r = normalizer
            .normalize(
                source,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            r.normalized_text(),
            "ö e\u{301}\r\non iki çarpı sekiz yüz otuz üç Türk lirası yirmi beş kuruş"
        );
        assert!(r.complete() && r.issues().is_empty() && r.fallbacks().is_empty());
        let mut cursor = 0;
        for part in r.segments() {
            assert_eq!(part.range().start(), cursor);
            cursor = part.range().end();
            assert!(boundaries.contains(&cursor));
            if part.kind() == SegmentKind::Verbatim {
                assert_eq!(part.text(), &source[part.range().start()..cursor]);
            }
        }
        assert_eq!(cursor, source.len());
        assert_eq!(
            r.segments().iter().map(|s| s.text()).collect::<String>(),
            r.normalized_text()
        );
    }
}
