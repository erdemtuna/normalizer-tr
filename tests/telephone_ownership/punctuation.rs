use super::support::{POLICIES, failure, options, partition};
use normalizer_tr::{
    AmbiguityPolicy, Hint, HintKind, NormalizeError, NormalizeOptions, Normalizer, SegmentKind,
    SourceRange,
};

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
fn nested_quote_membership_cannot_certify_an_invalid_phone_closer() {
    let normalizer = Normalizer::new().unwrap();
    let phone = "0532 123 45(67')')";
    let literal = "sıfır beş üç iki bir iki üç dört beş aç parantez altı yedi kesme kapat parantez kesme kapat parantez";
    for prefix in ["'‘(’' ", "O\u{308}nce '‘(’' "] {
        let source = format!("{prefix}{phone}; S ; 5 kg");
        let owner = SourceRange::new(prefix.len(), prefix.len() + phone.len());
        let (preserved, fallback) = failure(&normalizer, &source, owner);
        assert_eq!(
            preserved.normalized_text(),
            format!("{prefix}{phone}; se ; beş kilogram")
        );
        assert_eq!(
            fallback.normalized_text(),
            format!("{prefix}{literal}; se ; beş kilogram")
        );
        let letter = SourceRange::new(owner.end() + 2, owner.end() + 3);
        for result in [preserved, fallback] {
            let segment = result
                .segments()
                .iter()
                .find(|s| s.range() == letter)
                .unwrap();
            assert_eq!(segment.kind(), SegmentKind::Symbol);
            assert_eq!(segment.rule_id(), "letter.name");
            assert_eq!(segment.text(), "se");
        }
    }
}
