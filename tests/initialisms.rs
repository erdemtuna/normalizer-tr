//! Approved initialisms, pronunciation variants and protected source boundaries.
use normalizer_tr::{
    AmbiguityPolicy, Hint, HintKind, IssueCategory, LimitKind, NormalizeError, NormalizeOptions,
    Normalizer, SegmentKind, SourceRange,
};

const INITIALISMS: &[(&str, &str)] = &[
    ("CHP", "ce he pe"),
    ("AKP", "a ke pe"),
    ("TRT", "te re te"),
    ("SGK", "se ge ka"),
    ("YKS", "ye ke se"),
    ("GPU", "ge pe u"),
    ("ABD", "a be de"),
    ("AB", "a be"),
    ("USB", "u se be"),
    ("PDF", "pe de fe"),
    ("TC", "te ce"),
    ("AYM", "a ye me"),
    ("AİHM", "a i he me"),
    ("BDDK", "be de de ke"),
    ("BM", "be me"),
    ("BTK", "be te ke"),
    ("DSİ", "de se i"),
    ("DSÖ", "de se ö"),
    ("EGM", "e ge me"),
    ("İBB", "i be be"),
    ("İETT", "i e te te"),
    ("KVKK", "ke ve ke ke"),
    ("MHRS", "me he re se"),
    ("MSB", "me se be"),
    ("SPK", "se pe ke"),
    ("TCDD", "te ce de de"),
    ("TCK", "te ce ke"),
    ("TCMB", "te ce me be"),
    ("TDK", "te de ke"),
    ("THY", "te he ye"),
    ("TMSF", "te me se fe"),
    ("TSE", "te se e"),
    ("TSK", "te se ke"),
    ("YSK", "ye se ke"),
    ("KPSS", "ke pe se se"),
    ("LGS", "le ge se"),
    ("ÖSYM", "ö se ye me"),
    ("YDS", "ye de se"),
    ("İTÜ", "i te ü"),
    ("KTÜ", "ke te ü"),
    ("YTÜ", "ye te ü"),
    ("EFT", "e fe te"),
    ("ÖTV", "ö te ve"),
    ("BSMV", "be se me ve"),
    ("SSK", "se se ke"),
    ("CPU", "ce pe u"),
    ("API", "a pe i"),
    ("IP", "i pe"),
    ("HDMI", "he de me i"),
    ("URL", "u re le"),
    ("HTTP", "he te te pe"),
    ("HTTPS", "he te te pe se"),
    ("HTML", "he te me le"),
    ("XML", "iks me le"),
    ("CSS", "ce se se"),
    ("SMS", "se me se"),
    ("GPS", "ge pe se"),
    ("GSM", "ge se me"),
    ("DVD", "de ve de"),
    ("LCD", "le ce de"),
    ("TV", "te ve"),
    ("BBP", "be be pe"),
    ("DSP", "de se pe"),
    ("HDP", "he de pe"),
    ("MHP", "me he pe"),
];

const ALTERNATIVES: &[(&str, &str)] = &[
    ("SGK", "se ge ke"),
    ("BDDK", "be de de ka"),
    ("BTK", "be te ka"),
    ("KVKK", "ke ve ke ka"),
    ("SPK", "se pe ka"),
    ("TCK", "te ce ka"),
    ("TDK", "te de ka"),
    ("TSK", "te se ka"),
    ("YSK", "ye se ka"),
    ("SSK", "se se ka"),
    ("PDF", "pe de ef"),
];

const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

fn suffixes(reading: &str) -> [&str; 5] {
    if reading.ends_with(" ef") {
        ["i", "e", "te", "ten", "in"]
    } else {
        match reading.chars().next_back().unwrap() {
            'a' | 'ı' => ["yı", "ya", "da", "dan", "nın"],
            'o' | 'u' => ["yu", "ya", "da", "dan", "nun"],
            'ö' | 'ü' => ["yü", "ye", "de", "den", "nün"],
            'e' | 'i' => ["yi", "ye", "de", "den", "nin"],
            _ => panic!("unexpected golden pronunciation tail"),
        }
    }
}

fn primary(n: &Normalizer, source: &str, expected: &str) {
    for policy in POLICIES {
        let result = n
            .normalize(
                source,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(result.normalized_text(), expected, "{source} {policy:?}");
        assert!(result.complete() && result.issues().is_empty() && result.fallbacks().is_empty());
        assert_eq!(result.segments().len(), 1, "{source}");
        assert_eq!(result.segments()[0].kind(), SegmentKind::Abbreviation);
        assert_eq!(result.segments()[0].rule_id(), "abbreviation");
        assert_eq!(
            result.segments()[0].range(),
            SourceRange::new(0, source.len())
        );
    }
}

#[test]
fn every_approved_initialism_and_suffix_has_an_independent_reading() {
    assert_eq!(INITIALISMS.len(), 65);
    let n = Normalizer::new().unwrap();
    for &(symbol, reading) in INITIALISMS {
        primary(&n, symbol, reading);
        for suffix in suffixes(reading) {
            primary(
                &n,
                &format!("{symbol}'{suffix}"),
                &format!("{reading}{suffix}"),
            );
        }
    }
    for &(symbol, reading) in ALTERNATIVES {
        for suffix in suffixes(reading) {
            primary(
                &n,
                &format!("{symbol}’{suffix}"),
                &format!("{reading}{suffix}"),
            );
        }
    }
}

#[test]
fn approved_punctuation_keeps_source_partitions_and_spacing() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        ("CHP,AKP", "ce he pe,a ke pe"),
        ("SGK,YKS,GPU", "se ge ka,ye ke se,ge pe u"),
        ("TBMM,PTT", "te be me me,pe te te"),
        ("CHP,AKP;TRT:", "ce he pe,a ke pe;te re te:"),
        ("CHP,AKP;UNKNOWN", "ce he pe,a ke pe;UNKNOWN"),
        ("UNKNOWN;CHP,AKP", "UNKNOWN;ce he pe,a ke pe"),
        ("CHP,UNKNOWN;TRT,PDF", "CHP,UNKNOWN;te re te,pe de fe"),
        ("SGK'ya,PDF'ten:", "se ge kaya,pe de eften:"),
        ("LCD.", "le ce de."),
        ("XML.", "iks me le."),
        ("DVD.", "de ve de."),
        ("SGK'ya.", "se ge kaya."),
        ("TBMM'ye.", "te be me meye."),
        ("‘SGK’ya’", "‘se ge kaya’"),
        ("‘CHP,AKP’", "‘ce he pe,a ke pe’"),
        ("TRT:", "te re te:"),
        ("ÖSYM’ye", "ö se ye meye"),
        ("O\u{308}SYM’ye", "ö se ye meye"),
    ] {
        for policy in POLICIES {
            let result = n
                .normalize(
                    source,
                    &NormalizeOptions {
                        ambiguity_policy: policy,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(result.normalized_text(), expected, "{source}");
            assert!(result.complete() && !result.fallback_used());
            let mut cursor = 0;
            for segment in result.segments() {
                assert_eq!(segment.range().start(), cursor);
                cursor = segment.range().end();
                if segment.kind() == SegmentKind::Verbatim {
                    assert_eq!(segment.text(), &source[segment.range().start()..cursor]);
                }
            }
            assert_eq!(cursor, source.len());
            assert_eq!(
                result
                    .segments()
                    .iter()
                    .map(|segment| segment.text())
                    .collect::<String>(),
                expected
            );
        }
    }
}

#[test]
fn unknown_prose_and_ambiguous_lists_are_not_inferred() {
    let n = Normalizer::new().unwrap();
    for source in [
        "ABC",
        "SON DAKİKA",
        "ACİL DURUM",
        "sgk",
        "Sgk",
        "AIHM",
        "CHP,UNKNOWN",
        "CHP,,AKP",
        "CHP-AKP",
        "GB",
        "ATM",
        "NASA",
    ] {
        for policy in POLICIES {
            let result = n
                .normalize(
                    source,
                    &NormalizeOptions {
                        ambiguity_policy: policy,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(result.normalized_text(), source, "{source}");
            assert!(
                result.complete() && !result.fallback_used(),
                "{source} {policy:?}"
            );
            assert!(
                result
                    .segments()
                    .iter()
                    .all(|segment| segment.kind() == SegmentKind::Verbatim)
            );
        }
    }
}

#[test]
fn unmatched_suffixes_are_whole_findings_not_silent_repairs() {
    let n = Normalizer::new().unwrap();
    for (source, category) in [
        ("SGK'a", IssueCategory::InvalidExpression),
        ("PDF'lik", IssueCategory::InvalidExpression),
        ("SGK'ya'nın", IssueCategory::Unsupported),
        ("PDF'", IssueCategory::Unsupported),
    ] {
        let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(preserve.normalized_text(), source);
        assert!(!preserve.complete());
        assert_eq!(preserve.issues().len(), 1);
        assert_eq!(preserve.issues()[0].category(), category);
        assert_eq!(
            preserve.issues()[0].range(),
            SourceRange::new(0, source.len())
        );
        assert_eq!(
            n.normalize(
                source,
                &NormalizeOptions {
                    ambiguity_policy: AmbiguityPolicy::Reject,
                    ..Default::default()
                }
            ),
            Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
        );
        let fallback = n
            .normalize(
                source,
                &NormalizeOptions {
                    ambiguity_policy: AmbiguityPolicy::Fallback,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(fallback.complete() && fallback.fallback_used());
        assert_eq!(fallback.fallbacks().len(), 1);
        assert_eq!(fallback.fallbacks()[0].original_category(), Some(category));
        assert_eq!(
            fallback.fallbacks()[0].range(),
            SourceRange::new(0, source.len())
        );
    }
}

#[test]
fn invalid_list_members_keep_scoped_issues_and_complete_reject_collection() {
    let n = Normalizer::new().unwrap();
    let source = "SGK'a,PDF'lik,TRT";
    let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
    assert_eq!(preserve.normalized_text(), "SGK'a,PDF'lik,te re te");
    assert_eq!(
        preserve
            .issues()
            .iter()
            .map(|issue| issue.range())
            .collect::<Vec<_>>(),
        [SourceRange::new(0, 5), SourceRange::new(6, 13)]
    );
    assert_eq!(
        n.normalize(
            source,
            &NormalizeOptions {
                ambiguity_policy: AmbiguityPolicy::Reject,
                ..Default::default()
            }
        ),
        Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
    );
    let fallback = n
        .normalize(
            source,
            &NormalizeOptions {
                ambiguity_policy: AmbiguityPolicy::Fallback,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(fallback.complete() && fallback.issues().is_empty());
    assert_eq!(fallback.fallbacks().len(), 2);
    assert!(fallback.normalized_text().ends_with(",te re te"));
}

#[test]
fn protected_contexts_and_roman_intent_keep_priority() {
    let n = Normalizer::new().unwrap();
    for source in [
        "CHP@example.com",
        "https://ornek.com/TRT?x=PDF",
        "AB12(SGK,YKS)",
        "AB-12",
        "USB3",
        "GPU:123",
        "5 ATM",
        "5000 RPM",
        "IV",
        "CD",
        "I.",
        "V",
    ] {
        let result = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert!(
            result
                .segments()
                .iter()
                .all(|segment| segment.kind() != SegmentKind::Abbreviation),
            "{source}"
        );
    }
    for source in ["IV", "CD"] {
        let result = n
            .normalize(
                source,
                &NormalizeOptions {
                    hints: vec![Hint::new(
                        SourceRange::new(0, source.len()),
                        HintKind::Roman,
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(result.segments()[0].kind(), SegmentKind::Roman);
    }
    for source in ["LCD", "XML", "DVD"] {
        assert_eq!(
            n.normalize(
                source,
                &NormalizeOptions {
                    hints: vec![Hint::new(
                        SourceRange::new(0, source.len()),
                        HintKind::Roman
                    )],
                    ..Default::default()
                }
            ),
            Err(NormalizeError::InvalidHint)
        );
    }
}

#[test]
fn abbreviation_lists_obey_existing_candidate_limits() {
    let n = Normalizer::new().unwrap();
    let source = vec!["CHP"; normalizer_tr::MAX_CANDIDATES + 1].join(",");
    for policy in POLICIES {
        assert_eq!(
            n.normalize(
                &source,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    ..Default::default()
                }
            ),
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates))
        );
    }
}

#[test]
fn long_mixed_unknown_lists_remain_verbatim() {
    let n = Normalizer::new().unwrap();
    let source = vec!["CHP,UNKNOWN"; 1024].join(",");
    for policy in POLICIES {
        let result = n
            .normalize(
                &source,
                &NormalizeOptions {
                    ambiguity_policy: policy,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(result.normalized_text(), source);
        assert!(result.complete() && !result.fallback_used());
    }
}
