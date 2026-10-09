//! Accepted review regressions: bounded work and complete source ownership.
use normalizer_tr::{
    AmbiguityPolicy, Hint, HintKind, IssueCategory, NormalizeError, NormalizeOptions, Normalizer,
    SegmentKind, SourceRange, WorkControl,
};
use std::time::{Duration, Instant};

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

#[test]
fn malformed_comma_tokens_keep_outcomes_and_observe_controls() {
    let n = Normalizer::new().unwrap();
    for count in [2048, 8192, 16384] {
        let source = "1,".repeat(count);
        let preserve = n.normalize(&source, &NormalizeOptions::default()).unwrap();
        assert_eq!(preserve.normalized_text(), source);
        assert_eq!(preserve.issues().len(), 1);
        assert_eq!(
            preserve.issues()[0].range(),
            SourceRange::new(0, source.len() - 1)
        );
        assert_eq!(
            n.normalize(&source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
        );
        let fallback = n
            .normalize(&source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert_eq!(
            fallback.normalized_text(),
            format!("{},", vec!["bir"; count].join(" virgül "))
        );
        assert!(fallback.complete() && fallback.fallbacks().len() == 1);
        for policy in POLICIES {
            let expired = WorkControl::new(Some(Instant::now() - Duration::from_secs(1)));
            assert_eq!(
                n.normalize_controlled(&source, &options(policy), &expired),
                Err(NormalizeError::Cancelled)
            );
        }
    }
}

#[test]
fn cached_fallback_runs_keep_valid_remaining_suffix_preferences() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        ("1,1.234", "bir virgül bin iki yüz otuz dört"),
        ("1,234,50", "bir virgül iki yüz otuz dört virgül beş sıfır"),
        ("01,1.234", "sıfır bir virgül bin iki yüz otuz dört"),
        ("1.1.234", "bir nokta bin iki yüz otuz dört"),
        ("1,1,1", "bir virgül bir virgül bir"),
    ] {
        assert_eq!(
            n.normalize(source, &options(AmbiguityPolicy::Fallback))
                .unwrap()
                .normalized_text(),
            expected
        );
    }
    let padded = "0".repeat(normalizer_tr::MAX_INPUT_BYTES);
    let result = n
        .normalize(
            &padded,
            &NormalizeOptions {
                hints: vec![Hint::new(
                    SourceRange::new(0, padded.len()),
                    HintKind::Cardinal,
                )],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(result.normalized_text(), "sıfır");
    let grouped = "-999.999.999.999.999.999,123456789";
    let preserve = n.normalize(grouped, &NormalizeOptions::default()).unwrap();
    assert!(!preserve.complete());
    let fallback = n
        .normalize(grouped, &options(AmbiguityPolicy::Fallback))
        .unwrap();
    assert!(fallback.complete());
    assert!(fallback.normalized_text().starts_with("eksi"));
    let money = "-999\u{202f}999\u{202f}999\u{202f}999\u{202f}999\u{202f}999,99 TL";
    assert!(
        n.normalize(money, &NormalizeOptions::default())
            .unwrap()
            .complete()
    );
}

#[test]
fn duplicate_currency_markers_have_one_full_invalid_owner() {
    let n = Normalizer::new().unwrap();
    for source in [
        "$1 234,50TL",
        "$1 234,50USD",
        "$ 1 234,50USD",
        "$1 234,50$",
        "$1 234,50 TL",
        "USD 1 234,50TL",
        "€1\u{a0}234,50EUR",
        "₺1\u{202f}234,50TL'den",
        "$1 234 567,50USD",
        "$1 234,50USD'xx'yy",
    ] {
        let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(preserve.normalized_text(), source, "{source}");
        assert_eq!(preserve.issues().len(), 1);
        assert_eq!(
            preserve.issues()[0].category(),
            IssueCategory::InvalidExpression
        );
        assert_eq!(
            preserve.issues()[0].range(),
            SourceRange::new(0, source.len())
        );
        assert_eq!(preserve.segments().len(), 1);
        assert_eq!(preserve.segments()[0].kind(), SegmentKind::Unresolved);
        assert_eq!(
            n.normalize(source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
        );
        let fallback = n
            .normalize(source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert!(fallback.complete() && fallback.fallbacks().len() == 1);
        assert_eq!(
            fallback.fallbacks()[0].range(),
            SourceRange::new(0, source.len())
        );
        assert!(
            fallback
                .segments()
                .iter()
                .all(|segment| segment.kind() != SegmentKind::Money)
        );
    }
    for policy in POLICIES {
        let result = n.normalize("1 $25", &options(policy)).unwrap();
        assert_eq!(result.normalized_text(), "bir yirmi beş dolar");
        assert!(result.complete() && !result.fallback_used());
    }
}

#[test]
fn semicolon_members_do_not_inherit_another_members_digits() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        ("25kg;CHP,AKP", "yirmi beş kilogram;ce he pe,a ke pe"),
        ("CHP,AKP;25TL", "ce he pe,a ke pe;yirmi beş Türk lirası"),
        (
            "25TL;SGK'ya,PDF'ten:",
            "yirmi beş Türk lirası;se ge kaya,pe de eften:",
        ),
        (
            "25kg;Claude,Hugging Face",
            "yirmi beş kilogram;klod,haging feys",
        ),
    ] {
        for policy in POLICIES {
            let result = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(result.normalized_text(), expected, "{source} {policy:?}");
            assert!(result.complete() && !result.fallback_used());
        }
    }
}

#[test]
fn a_later_unknown_or_empty_member_prevents_every_primary_catalog_rewrite() {
    let n = Normalizer::new().unwrap();
    for source in [
        "Claude,Hugging Face,UNKNOWN",
        "Claude,Visual Studio Code,UNKNOWN",
        "UNKNOWN,Hugging Face,Claude",
        "Claude,Hugging Face,,",
        "Claude,,Hugging Face",
        "Claude,Visual Studio Code,,UNKNOWN",
        "foo GitHub Copilot,UNKNOWN",
        "Claude,Hugging Face,GitHub Copilot,UNKNOWN",
        "Claude'a,Hugging Face,UNKNOWN",
        "SGK'ya,UNKNOWN",
    ] {
        for policy in POLICIES {
            let result = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(result.normalized_text(), source, "{source} {policy:?}");
            assert!(result.complete() && !result.fallback_used());
            assert!(
                result
                    .segments()
                    .iter()
                    .all(|segment| segment.kind() == SegmentKind::Verbatim)
            );
        }
    }
    for source in ["Claude,Hugging Face,AB12", "AB12,Hugging Face,Claude"] {
        let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(preserve.normalized_text(), source);
        assert!(!preserve.complete());
        assert!(
            preserve
                .issues()
                .iter()
                .any(|issue| issue.category() == IssueCategory::ProtectedIdentifier)
        );
        assert_eq!(
            n.normalize(source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
        );
        let fallback = n
            .normalize(source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert!(fallback.complete() && fallback.fallback_used());
        assert!(fallback.segments().iter().all(|segment| !matches!(
            segment.kind(),
            SegmentKind::Pronunciation | SegmentKind::Abbreviation
        )));
    }
}

#[test]
fn fully_known_groups_and_spaced_prose_keep_their_distinct_behavior() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        (
            "Claude,Hugging Face,GitHub Copilot",
            "klod,haging feys,git hab ko paylıt",
        ),
        (
            "Claude,Visual Studio Code,VS Code",
            "klod,vijuıl stüdyo kod,vi es kod",
        ),
        (
            "Claude, Hugging Face, UNKNOWN",
            "klod, haging feys, UNKNOWN",
        ),
        ("Claude,Hugging Face;UNKNOWN", "klod,haging feys;UNKNOWN"),
        (
            "‘Claude,Hugging Face,UNKNOWN’",
            "‘Claude,Hugging Face,UNKNOWN’",
        ),
        ("‘Claude,Hugging Face’", "‘klod,haging feys’"),
        ("Claude,Hugging Face,", "klod,haging feys,"),
        (
            "o\u{308} Claude,Hugging Face,UNKNOWN!",
            "o\u{308} Claude,Hugging Face,UNKNOWN!",
        ),
    ] {
        for policy in POLICIES {
            let result = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(result.normalized_text(), expected, "{source} {policy:?}");
            assert!(result.complete() && !result.fallback_used());
            assert_eq!(
                result
                    .segments()
                    .iter()
                    .map(|segment| segment.text())
                    .collect::<String>(),
                expected
            );
            let mut cursor = 0;
            for segment in result.segments() {
                assert_eq!(segment.range().start(), cursor);
                cursor = segment.range().end();
                assert!(source.is_char_boundary(cursor));
            }
            assert_eq!(cursor, source.len());
        }
    }
}
