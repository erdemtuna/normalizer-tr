use super::support::{PHONE, POLICIES, SPOKEN, partition};
use normalizer_tr::{
    AmbiguityPolicy, Hint, HintKind, NormalizeError, NormalizeOptions, Normalizer, SegmentKind,
    SourceRange,
};

#[test]
fn telephone_hints_end_before_a_released_period() {
    let normalizer = Normalizer::new().unwrap();
    let prefix = "O\u{308}nce ";
    let phone = "0532 123 45 67";
    let source = format!("{prefix}{phone}.");
    let end = prefix.len() + phone.len();
    let range = SourceRange::new(prefix.len(), end);
    for policy in POLICIES {
        let opts = |range, kind| NormalizeOptions {
            ambiguity_policy: policy,
            hints: vec![Hint::new(range, kind)],
        };
        let result = normalizer
            .normalize(&source, &opts(range, HintKind::Telephone))
            .unwrap();
        assert_eq!(result.normalized_text(), format!("{prefix}{SPOKEN}."));
        assert_eq!(result.segments()[1].range(), range);
        assert_eq!(result.segments()[1].kind(), SegmentKind::Telephone);
        partition(&source, &result);
        for incorrect_end in [end - 1, end + 1] {
            assert_eq!(
                normalizer.normalize(
                    &source,
                    &opts(
                        SourceRange::new(prefix.len(), incorrect_end),
                        HintKind::Telephone
                    )
                ),
                Err(NormalizeError::InvalidHint)
            );
        }
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
