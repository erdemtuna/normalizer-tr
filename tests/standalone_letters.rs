//! Primary standalone glyph names, source eligibility and stronger owners.
use std::time::{Duration, Instant};

use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, Hint, HintKind, IssueCategory, LimitKind,
    MAX_CANDIDATES, NormalizeError, NormalizeOptions, NormalizeResult, Normalizer, SegmentKind,
    SourceRange, WorkControl,
};
use unicode_segmentation::UnicodeSegmentation;

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

fn partition(source: &str, result: &NormalizeResult) {
    let boundaries: Vec<_> = source
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(std::iter::once(source.len()))
        .collect();
    let mut cursor = 0;
    for segment in result.segments() {
        assert_eq!(segment.range().start(), cursor);
        cursor = segment.range().end();
        assert!(boundaries.contains(&cursor));
        if matches!(
            segment.kind(),
            SegmentKind::Verbatim | SegmentKind::Unresolved
        ) {
            assert_eq!(segment.text(), &source[segment.range().start()..cursor]);
        }
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
    assert_eq!(result.complete(), result.issues().is_empty());
    assert_eq!(result.fallback_used(), !result.fallbacks().is_empty());
    for range in result
        .issues()
        .iter()
        .map(|i| i.range())
        .chain(result.fallbacks().iter().map(|f| f.range()))
    {
        assert!(range.start() < range.end());
        assert!(boundaries.contains(&range.start()) && boundaries.contains(&range.end()));
    }
}

#[test]
fn every_supported_uppercase_glyph_is_primary_in_every_policy() {
    let n = Normalizer::new().unwrap();
    for (letter, name) in [
        ('A', "a"),
        ('B', "be"),
        ('C', "ce"),
        ('Ç', "çe"),
        ('D', "de"),
        ('E', "e"),
        ('F', "fe"),
        ('G', "ge"),
        ('Ğ', "yumuşak ge"),
        ('H', "he"),
        ('I', "ı"),
        ('İ', "i"),
        ('J', "je"),
        ('K', "ke"),
        ('L', "le"),
        ('M', "me"),
        ('N', "ne"),
        ('O', "o"),
        ('Ö', "ö"),
        ('P', "pe"),
        ('Q', "kü"),
        ('R', "re"),
        ('S', "se"),
        ('Ş', "şe"),
        ('T', "te"),
        ('U', "u"),
        ('Ü', "ü"),
        ('V', "ve"),
        ('W', "çift ve"),
        ('X', "iks"),
        ('Y', "ye"),
        ('Z', "ze"),
    ] {
        let source = letter.to_string();
        for policy in POLICIES {
            let r = n.normalize(&source, &options(policy)).unwrap();
            assert_eq!(r.normalized_text(), name);
            assert!(r.complete() && r.issues().is_empty() && r.fallbacks().is_empty());
            assert_eq!(r.segments().len(), 1);
            assert_eq!(r.segments()[0].kind(), SegmentKind::Symbol);
            assert_eq!(r.segments()[0].rule_id(), "letter.name");
            assert_eq!(r.segments()[0].range(), SourceRange::new(0, source.len()));
            partition(&source, &r);
        }
    }
}

#[test]
fn raw_whitespace_and_confirmed_quotes_keep_original_partitions() {
    let n = Normalizer::new().unwrap();
    for (source, expected, letters) in [
        ("P ve S dalgaları", "pe ve se dalgaları", 2),
        ("P ve S ve T dalgaları", "pe ve se ve te dalgaları", 3),
        ("S Yılmaz; O geldi.", "se Yılmaz; o geldi.", 2),
        ("P \t\r\n\u{a0}\u{202f}S", "pe \t\r\n\u{a0}\u{202f}se", 2),
        ("\"P ve S dalgaları\"", "\"pe ve se dalgaları\"", 2),
        ("'P ve S dalgaları'", "'pe ve se dalgaları'", 2),
        ("“P ve S dalgaları”", "“pe ve se dalgaları”", 2),
        ("‘P ve S dalgaları’", "‘pe ve se dalgaları’", 2),
        ("“P” ve ‘S’ dalgaları", "“pe” ve ‘se’ dalgaları", 2),
        ("“‘P’ ve ‘S’ dalgaları”", "“‘pe’ ve ‘se’ dalgaları”", 2),
        ("‘\"P\" ve \"S\"’", "‘\"pe\" ve \"se\"’", 2),
        ("\"S'nin ve P\"", "\"S'nin ve pe\"", 1),
        ("P, S", "P, se", 1),
        ("ö I\u{307} C\u{327} S\u{327}", "ö i çe şe", 3),
    ] {
        for policy in POLICIES {
            let r = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(r.normalized_text(), expected, "{source} {policy:?}");
            assert!(r.complete() && r.fallbacks().is_empty());
            let named: Vec<_> = r
                .segments()
                .iter()
                .filter(|s| s.rule_id() == "letter.name")
                .collect();
            assert_eq!(named.len(), letters, "{source}");
            assert!(named.iter().all(|s| s.kind() == SegmentKind::Symbol));
            partition(source, &r);
        }
    }
    let source = "ö I\u{307} S";
    let r = n.normalize(source, &NormalizeOptions::default()).unwrap();
    assert_eq!(r.segments()[1].range(), SourceRange::new(3, 6));
    assert_eq!(r.segments()[3].range(), SourceRange::new(7, 8));
}

#[test]
fn trimming_never_exposes_a_new_letter_reading() {
    let n = Normalizer::new().unwrap();
    for source in [
        "S.",
        "S,",
        "S:",
        "S)",
        "(S)",
        "[S]",
        "P,S",
        "S/P",
        "S-P",
        "S_",
        "S;T",
        "S)P",
        "(S)P",
        "S'nin",
        "S’nin",
        "\"S",
        "“S",
        "‘S",
        "S\"",
        "S”",
        "“S’",
        "‘S”",
        "''S''",
        "“S”P",
        "P“S”",
        "«S»",
        "S\u{200b}P",
        "S\u{301}",
        "e o a i ı",
        "ABC BUGÜN",
        "α Ω",
    ] {
        for policy in POLICIES {
            let r = n.normalize(source, &options(policy)).unwrap();
            assert!(
                r.segments().iter().all(|s| s.rule_id() != "letter.name"),
                "{source}"
            );
            partition(source, &r);
            if policy != AmbiguityPolicy::Fallback
                || !source.contains(['/', '_', '\u{200b}', '«', '»'])
            {
                assert_eq!(r.normalized_text(), source, "{source}");
                assert!(r.complete() && r.fallbacks().is_empty());
            }
        }
    }
}

#[test]
fn existing_owners_and_hints_remain_authoritative() {
    let n = Normalizer::new().unwrap();
    for source in [
        "5 V",
        "01 V",
        "A 5",
        "W 5",
        "V x 2",
        "1 x S",
        "S123",
        "AB12(S)",
        "https://ornek.com/S?x=P",
        "S@ornek.com",
        "adres No:S Kat:3 D:4",
        "SGK'ya",
        "V. yüzyıl",
        "IV",
        "IIII",
        "C.",
    ] {
        let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
        for policy in POLICIES {
            if policy == AmbiguityPolicy::Reject && !preserve.complete() {
                assert_eq!(
                    n.normalize(source, &options(policy)),
                    Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
                );
                continue;
            }
            let r = n.normalize(source, &options(policy)).unwrap();
            assert!(
                r.segments().iter().all(|s| s.rule_id() != "letter.name"),
                "{source}"
            );
            partition(source, &r);
        }
    }

    for policy in POLICIES {
        let mut opts = options(policy);
        opts.hints
            .push(Hint::new(SourceRange::new(0, 1), HintKind::Roman));
        let r = n.normalize("V yaka", &opts).unwrap();
        assert_eq!(r.normalized_text(), "beş yaka");
        assert_eq!(r.segments()[0].kind(), SegmentKind::Roman);
        opts.hints = vec![Hint::new(SourceRange::new(3, 4), HintKind::Roman)];
        let quoted = n.normalize("“V”", &opts).unwrap();
        assert_eq!(quoted.normalized_text(), "“beş”");
        assert_eq!(quoted.segments()[1].kind(), SegmentKind::Roman);
        partition("“V”", &quoted);
        for (source, range) in [
            ("S123", SourceRange::new(1, 4)),
            ("01 V", SourceRange::new(0, 2)),
            ("1 x S", SourceRange::new(4, 5)),
            ("I\u{307}", SourceRange::new(0, 1)),
        ] {
            opts.hints = vec![Hint::new(range, HintKind::Cardinal)];
            assert_eq!(n.normalize(source, &opts), Err(NormalizeError::InvalidHint));
        }
        let r = n.normalize("V 5", &options(policy)).unwrap();
        assert_eq!(r.normalized_text(), "ve beş");
        assert_eq!(r.segments()[0].rule_id(), "letter.name");
        assert_eq!(r.segments()[2].kind(), SegmentKind::Cardinal);
    }
}

#[test]
fn independent_names_do_not_hide_unrelated_findings() {
    let n = Normalizer::new().unwrap();
    for (source, owner, class, category, reason) in [
        (
            "01 S",
            SourceRange::new(0, 2),
            FallbackClass::Number,
            IssueCategory::InvalidExpression,
            FallbackReason::LeadingZeroes,
        ),
        (
            "adres No: P ve S dalgaları Kat:3 D:4",
            SourceRange::new(6, 11),
            FallbackClass::Number,
            IssueCategory::InvalidExpression,
            FallbackReason::InvalidForm,
        ),
    ] {
        let preserve = n
            .normalize(source, &options(AmbiguityPolicy::Preserve))
            .unwrap();
        assert!(!preserve.complete());
        assert_eq!(preserve.issues().len(), 1);
        assert_eq!(preserve.issues()[0].range(), owner);
        assert_eq!(preserve.issues()[0].category(), category);
        assert_eq!(
            preserve
                .segments()
                .iter()
                .filter(|s| s.rule_id() == "letter.name")
                .count(),
            1
        );
        assert_eq!(
            n.normalize(source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
        );
        let fallback = n
            .normalize(source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert!(fallback.complete() && fallback.issues().is_empty());
        assert_eq!(fallback.fallbacks().len(), 1);
        let record = &fallback.fallbacks()[0];
        assert_eq!(record.range(), owner);
        assert_eq!(record.attempted_class(), class);
        assert_eq!(record.reason(), reason);
        assert_eq!(record.strategy(), normalizer_tr::FallbackStrategy::Literal);
        assert_eq!(record.original_category(), Some(category));
        partition(source, &preserve);
        partition(source, &fallback);
    }
}

#[test]
fn amplification_and_controls_remain_engineering_errors() {
    let n = Normalizer::new().unwrap();
    let too_many = vec!["S"; MAX_CANDIDATES + 1].join(" ");
    let amplified = vec!["W"; MAX_CANDIDATES].join(" ");
    for policy in POLICIES {
        assert_eq!(
            n.normalize(&too_many, &options(policy)),
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates))
        );
        assert_eq!(
            n.normalize(&amplified, &options(policy)),
            Err(NormalizeError::LimitExceeded(LimitKind::Result))
        );
        let control = WorkControl::default();
        control.cancel();
        assert_eq!(
            n.normalize_controlled("“S”", &options(policy), &control),
            Err(NormalizeError::Cancelled)
        );
        let expired = WorkControl::new(Some(Instant::now() - Duration::from_secs(1)));
        assert_eq!(
            n.normalize_controlled("“S”", &options(policy), &expired),
            Err(NormalizeError::Cancelled)
        );
        for source in [
            "\"".repeat(16_000),
            format!("\"{}", "S ".repeat(1_000)),
            "S".repeat(32_768),
        ] {
            let r = n.normalize(&source, &options(policy)).unwrap();
            assert!(r.complete());
            partition(&source, &r);
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn primary_letter_serializes_as_existing_symbol_without_fallback_provenance() {
    let r = Normalizer::new()
        .unwrap()
        .normalize("I", &NormalizeOptions::default())
        .unwrap();
    let record = serde_json::to_value(r).unwrap();
    assert_eq!(record["normalized_text"], "ı");
    assert_eq!(record["segments"][0]["kind"], "Symbol");
    assert_eq!(record["segments"][0]["rule_id"], "letter.name");
    assert_eq!(
        record["segments"][0]["range"],
        serde_json::json!({"start": 0, "end": 1})
    );
    assert_eq!(record["issues"], serde_json::json!([]));
    assert_eq!(record["fallbacks"], serde_json::json!([]));
    assert_eq!(record["complete"], true);
    assert_eq!(record["fallback_used"], false);
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(128))]
    #[test]
    fn independent_source_atoms_and_wrappers_preserve_every_byte(
        separator in proptest::sample::select(vec![" ", "\t", "\n", "\r\n", "\u{a0}", "\u{202f}", " \t "]),
        wrapper in proptest::sample::select(vec![("", ""), ("'", "'"), ("\"", "\""), ("‘", "’"), ("“", "”")]),
        count in 1_usize..32,
    ) {
        let source = (0..count).map(|_| format!("{}S{}", wrapper.0, wrapper.1)).collect::<Vec<_>>().join(separator);
        let expected = (0..count).map(|_| format!("{}se{}", wrapper.0, wrapper.1)).collect::<Vec<_>>().join(separator);
        let n = Normalizer::new().unwrap();
        for policy in POLICIES {
            let result = n.normalize(&source, &options(policy)).unwrap();
            proptest::prop_assert_eq!(result.normalized_text(), &expected);
            proptest::prop_assert!(result.complete() && !result.fallback_used());
            proptest::prop_assert_eq!(result.segments().iter().filter(|segment| segment.rule_id() == "letter.name").count(), count);
            partition(&source, &result);
        }
    }
}
