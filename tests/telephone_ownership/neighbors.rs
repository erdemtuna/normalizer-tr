use super::support::{
    LITERAL, PHONE, POLICIES, failure, failure_with_strategy, options, partition,
};
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, Hint, HintKind,
    IssueCategory, NormalizeError, Normalizer, SegmentKind, SourceRange,
};

#[test]
fn terminal_singletons_keep_exact_claims_before_queued_address_fields() {
    let normalizer = Normalizer::new().unwrap();
    for prefix in ["", "O\u{308}nce "] {
        let before = format!("{prefix}adres No:5 I ");
        let source = format!("{before}{PHONE} Kat:3 S");
        let owner = SourceRange::new(before.len(), before.len() + PHONE.len());
        let (preserved, fallback) = failure(&normalizer, &source, owner);
        for (result, phone) in [(&preserved, PHONE), (&fallback, LITERAL)] {
            assert_eq!(
                result.normalized_text(),
                format!("{prefix}adres numara beş ı {phone} kat üç se")
            );
            let letters: Vec<_> = result
                .segments()
                .iter()
                .filter(|s| s.rule_id() == "letter.name")
                .collect();
            assert_eq!(letters.len(), 2);
            assert_eq!(letters[0].kind(), SegmentKind::Symbol);
            assert_eq!(
                letters[0].range(),
                SourceRange::new(before.len() - 2, before.len() - 1)
            );
            assert_eq!(
                letters[1].range(),
                SourceRange::new(source.len() - 1, source.len())
            );
        }
        for policy in POLICIES {
            let mut opts = options(policy);
            opts.hints = vec![Hint::new(
                SourceRange::new(before.len() - 2, before.len() - 1),
                HintKind::Roman,
            )];
            if policy == AmbiguityPolicy::Reject {
                assert_eq!(
                    normalizer.normalize(&source, &opts),
                    Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
                );
            } else {
                let hinted = normalizer.normalize(&source, &opts).unwrap();
                let phone = if policy == AmbiguityPolicy::Fallback {
                    LITERAL
                } else {
                    PHONE
                };
                assert_eq!(
                    hinted.normalized_text(),
                    format!("{prefix}adres numara beş bir {phone} kat üç se")
                );
                partition(&source, &hinted);
            }
            opts.hints = vec![Hint::new(
                SourceRange::new(owner.start(), owner.end() - 1),
                HintKind::Digits,
            )];
            assert_eq!(
                normalizer.normalize(&source, &opts),
                Err(NormalizeError::InvalidHint)
            );
        }
    }
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
