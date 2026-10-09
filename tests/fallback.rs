//! Source-faithful fallback policy acceptance contract.
use normalizer_tr::{
    AmbiguityPolicy, FallbackStrategy, Hint, HintKind, NormalizeError, NormalizeOptions,
    Normalizer, SegmentKind, SourceRange, WorkControl,
};

fn options() -> NormalizeOptions {
    NormalizeOptions {
        ambiguity_policy: AmbiguityPolicy::Fallback,
        ..Default::default()
    }
}

#[test]
fn fallback_readings_are_source_faithful_and_diagnostic() {
    let normalizer = Normalizer::new().unwrap();
    for (input, expected) in [
        ("1.234", "bin iki yüz otuz dört"),
        ("00042", "sıfır sıfır sıfır dört iki"),
        ("AB12", "a be bir iki"),
        ("IV", "ı ve"),
        ("Toplam 25.", "Toplam yirmi beş."),
        ("10-15", "on tire on beş"),
        ("1/2", "bir eğik çizgi iki"),
        ("12:30", "on iki otuz"),
        ("40.03.2026", "kırk Mart iki bin yirmi altı"),
        (
            "tarih 31.02.2026",
            "tarih otuz bir Şubat iki bin yirmi altı",
        ),
    ] {
        let result = normalizer.normalize(input, &options()).unwrap();
        assert_eq!(result.normalized_text(), expected, "{input}");
        assert!(result.complete(), "{input}");
        assert!(result.issues().is_empty(), "{input}");
        assert!(result.fallback_used(), "{input}");
        assert!(!result.fallbacks().is_empty(), "{input}");
        assert!(
            result
                .segments()
                .iter()
                .any(|segment| segment.kind() == SegmentKind::Fallback)
        );
        assert_eq!(
            result
                .segments()
                .iter()
                .map(|s| s.text())
                .collect::<String>(),
            result.normalized_text()
        );
        assert_eq!(result.segments()[0].range().start(), 0);
        assert_eq!(result.segments().last().unwrap().range().end(), input.len());
    }
}

#[test]
fn normal_readings_and_valid_hints_win() {
    let normalizer = Normalizer::new().unwrap();
    for input in [
        "25 TL",
        "4,25",
        "saat 12:05",
        "%37,5'lik",
        "II. Dünya Savaşı",
        "0850 222 33 44",
        "https://ornek.com/a@b?x=2",
    ] {
        let primary = normalizer
            .normalize(input, &NormalizeOptions::default())
            .unwrap();
        assert!(primary.complete());
        let fallback = normalizer.normalize(input, &options()).unwrap();
        assert_eq!(fallback.normalized_text(), primary.normalized_text());
        assert_eq!(fallback.segments(), primary.segments());
        assert!(!fallback.fallback_used());
    }
    let mut hinted = options();
    hinted
        .hints
        .push(Hint::new(SourceRange::new(0, 2), HintKind::Roman));
    let result = normalizer.normalize("IV", &hinted).unwrap();
    assert_eq!(result.normalized_text(), "dört");
    assert!(!result.fallback_used());
}

#[test]
fn malformed_identifiers_have_one_claim_and_no_fragment_certification() {
    let normalizer = Normalizer::new().unwrap();
    for input in [
        "https://name:pw@ornek.com/x%ZZ",
        "AB0012",
        "Ж12",
        "abc_0002",
    ] {
        let result = normalizer.normalize(input, &options()).unwrap();
        assert!(result.complete());
        assert_eq!(result.fallbacks().len(), 1, "{input}");
        assert_eq!(
            result.fallbacks()[0].range(),
            SourceRange::new(0, input.len())
        );
        assert_eq!(result.segments().len(), 1);
        assert_eq!(result.segments()[0].kind(), SegmentKind::Fallback);
        assert!(!result.normalized_text().contains("%ZZ"));
    }
    let result = normalizer.normalize("AB0012", &options()).unwrap();
    assert_eq!(result.normalized_text(), "a be sıfır sıfır bir iki");
    let result = normalizer
        .normalize("προ Hello q\u{301}", &options())
        .unwrap();
    assert_eq!(result.normalized_text(), "προ Hello q\u{301}");
    assert!(!result.fallback_used());
}

#[test]
fn uncovered_symbols_are_rendered_without_global_word_rewriting() {
    let normalizer = Normalizer::new().unwrap();
    let result = normalizer.normalize("hello🙂world", &options()).unwrap();
    assert_eq!(result.normalized_text(), "hello gülümseyen yüz world");
    assert_eq!(result.fallbacks()[0].range(), SourceRange::new(5, 9));
    let result = normalizer.normalize("🫠", &options()).unwrap();
    assert_eq!(result.normalized_text(), "unikod u artı bir fe a e sıfır");
    assert_eq!(
        result.fallbacks()[0].strategy(),
        FallbackStrategy::UnicodeCodePoint
    );
    assert!(result.complete());
    for input in ["→", "\\", "|", "✓", "<3", ":D", "***"] {
        let result = normalizer.normalize(input, &options()).unwrap();
        assert!(result.complete(), "{input}");
        assert!(!result.normalized_text().is_empty(), "{input}");
        assert!(result.fallback_used(), "{input}");
    }
}

#[test]
fn errors_remain_errors_and_hints_cannot_be_ignored() {
    let normalizer = Normalizer::new().unwrap();
    for input in ["", " ", "a\u{202e}12", "a\u{200f}b"] {
        assert_eq!(
            normalizer.normalize(input, &options()),
            Err(NormalizeError::InvalidInput)
        );
    }
    let mut invalid_hint = options();
    invalid_hint
        .hints
        .push(Hint::new(SourceRange::new(0, 1), HintKind::Cardinal));
    assert_eq!(
        normalizer.normalize("3 + 4", &invalid_hint),
        Err(NormalizeError::InvalidHint)
    );
    let control = WorkControl::default();
    control.cancel();
    assert_eq!(
        normalizer.normalize_controlled("🙂", &options(), &control),
        Err(NormalizeError::Cancelled)
    );
}

#[test]
fn default_partial_contract_is_unchanged() {
    let result = Normalizer::new()
        .unwrap()
        .normalize("25 TL; 1.234", &NormalizeOptions::default())
        .unwrap();
    assert_eq!(result.normalized_text(), "yirmi beş Türk lirası; 1.234");
    assert!(!result.complete());
    assert!(!result.fallback_used());
    assert!(result.fallbacks().is_empty());
}

#[test]
fn validated_temporal_preferences_reuse_normal_rendering_and_inflection() {
    let normalizer = Normalizer::new().unwrap();
    for (input, cued, strategy) in [
        ("12:00", "saat 12:00", FallbackStrategy::PreferredTime),
        ("09:30'da", "saat 09:30'da", FallbackStrategy::PreferredTime),
        (
            "14.03.2026'da",
            "tarih 14.03.2026'da",
            FallbackStrategy::PreferredDate,
        ),
    ] {
        let ordinary = normalizer
            .normalize(cued, &NormalizeOptions::default())
            .unwrap();
        let result = normalizer.normalize(input, &options()).unwrap();
        let expected = ordinary.normalized_text().split_once(' ').unwrap().1;
        assert_eq!(result.normalized_text(), expected, "{input}");
        assert_eq!(result.fallbacks()[0].strategy(), strategy);
        assert!(result.complete());
    }
}

#[test]
fn invalid_values_are_not_repaired_or_certified() {
    use normalizer_tr::{FallbackClass, FallbackReason, IssueCategory};
    let normalizer = Normalizer::new().unwrap();
    for (input, expected, class, strategy) in [
        (
            "saat 25:70",
            "saat yirmi beş yetmiş",
            FallbackClass::Time,
            FallbackStrategy::SurfaceTime,
        ),
        (
            "2026-03-40",
            "kırk Mart iki bin yirmi altı",
            FallbackClass::Date,
            FallbackStrategy::SurfaceDate,
        ),
        (
            "tarih 31.13.2026",
            "tarih otuz bir nokta on üç nokta iki bin yirmi altı",
            FallbackClass::Date,
            FallbackStrategy::Literal,
        ),
        (
            "25,123 TL",
            "yirmi beş virgül bir iki üç Türk lirası",
            FallbackClass::Quantity,
            FallbackStrategy::Literal,
        ),
        (
            "1.234'te",
            "bin iki yüz otuz dörtte",
            FallbackClass::Number,
            FallbackStrategy::PreferredNumber,
        ),
        (
            "1.234'dan",
            "bin iki yüz otuz dört kesme dan",
            FallbackClass::Number,
            FallbackStrategy::Literal,
        ),
    ] {
        let result = normalizer.normalize(input, &options()).unwrap();
        assert_eq!(result.normalized_text(), expected, "{input}");
        assert!(result.complete());
        assert_eq!(result.fallbacks().len(), 1);
        assert_eq!(result.fallbacks()[0].attempted_class(), class, "{input}");
        assert_eq!(result.fallbacks()[0].strategy(), strategy, "{input}");
    }
    let input = "TR00 0006 1005 1978 6457 8413 26";
    let result = normalizer.normalize(input, &options()).unwrap();
    assert_eq!(
        result.normalized_text(),
        "te re sıfır sıfır sıfır sıfır sıfır altı bir sıfır sıfır beş bir dokuz yedi sekiz altı dört beş yedi sekiz dört bir üç iki altı"
    );
    assert_eq!(
        result.fallbacks()[0].range(),
        SourceRange::new(0, input.len())
    );
    assert_eq!(
        result.fallbacks()[0].original_category(),
        Some(IssueCategory::ProtectedIdentifier)
    );
    assert_eq!(
        result.fallbacks()[0].reason(),
        FallbackReason::ProtectedIdentifier
    );
    assert_eq!(result.segments()[0].kind(), SegmentKind::Fallback);
    assert_ne!(result.segments()[0].rule_id(), "iban.tr.mod97");
}

#[test]
fn graphemes_nfc_crlf_and_normal_punctuation_keep_original_coordinates() {
    use unicode_segmentation::UnicodeSegmentation;
    let normalizer = Normalizer::new().unwrap();
    for input in [
        "o\u{308}🙂\r\nABC",
        "👨‍👩‍👧",
        "🙂\u{fe0f}",
        ":D\u{301}",
        "q\u{301}🙂",
        "\u{301}",
        "a\u{200b}b",
        "I\u{307}12",
    ] {
        let result = normalizer.normalize(input, &options()).unwrap();
        let mut boundaries: Vec<_> = input
            .grapheme_indices(true)
            .map(|(start, _)| start)
            .collect();
        boundaries.push(input.len());
        let mut end = 0;
        for segment in result.segments() {
            assert_eq!(segment.range().start(), end);
            end = segment.range().end();
            assert!(boundaries.contains(&end), "{input:?}");
            assert!(!segment.text().is_empty(), "{input:?}");
        }
        assert_eq!(end, input.len());
        assert!(result.complete());
        assert!(result.fallback_used());
    }
    let result = normalizer.normalize("o\u{308}🙂!", &options()).unwrap();
    assert_eq!(result.normalized_text(), "o\u{308} gülümseyen yüz!");
    assert_eq!(result.fallbacks()[0].range(), SourceRange::new(3, 7));
    let result = normalizer
        .normalize("Söz: (merhaba), \"evet\"!\r\n", &options())
        .unwrap();
    assert_eq!(result.normalized_text(), "Söz: (merhaba), \"evet\"!\r\n");
    assert!(!result.fallback_used());
    let result = normalizer.normalize("π", &options()).unwrap();
    assert_eq!(result.normalized_text(), "pi");
    assert_eq!(result.fallbacks()[0].strategy(), FallbackStrategy::Literal);
}

#[test]
fn fallback_limits_deadlines_and_concurrency_are_real_controls() {
    use normalizer_tr::{LimitKind, MAX_INPUT_BYTES};
    use std::time::Instant;
    let normalizer = Normalizer::new().unwrap();
    assert_eq!(
        normalizer.normalize(&"a".repeat(MAX_INPUT_BYTES + 1), &options()),
        Err(NormalizeError::LimitExceeded(LimitKind::Input))
    );
    assert_eq!(
        normalizer.normalize(&"🙂 ".repeat(4097), &options()),
        Err(NormalizeError::LimitExceeded(LimitKind::Candidates))
    );
    assert_eq!(
        normalizer.normalize(&"🫠".repeat(MAX_INPUT_BYTES / 4), &options()),
        Err(NormalizeError::LimitExceeded(LimitKind::Result))
    );
    assert_eq!(
        normalizer.normalize_controlled(
            "ABC🙂",
            &options(),
            &WorkControl::new(Some(Instant::now()))
        ),
        Err(NormalizeError::Cancelled)
    );
    let expected = normalizer
        .normalize("AB0012; 🫠; 40.03.2026", &options())
        .unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let expected = &expected;
            let normalizer = &normalizer;
            scope.spawn(move || {
                for _ in 0..20 {
                    assert_eq!(
                        normalizer
                            .normalize("AB0012; 🫠; 40.03.2026", &options())
                            .unwrap(),
                        *expected
                    );
                }
            });
        }
    });
}

#[cfg(feature = "serde")]
#[test]
fn serialized_fallback_contract_has_no_parallel_completion_flag() {
    let result = Normalizer::new()
        .unwrap()
        .normalize("🙂", &options())
        .unwrap();
    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized["complete"], true);
    assert_eq!(serialized["fallback_used"], true);
    assert_eq!(serialized["issues"], serde_json::json!([]));
    assert_eq!(
        serialized["fallbacks"][0],
        serde_json::json!({
            "range":{"start":0,"end":4}, "attempted_class":"Symbol", "reason":"UnhandledSymbol",
            "original_category":null, "strategy":"Literal"
        })
    );
    assert!(serialized.get("fully_rendered").is_none());
    assert_eq!(serialized["segments"][0]["kind"], "Fallback");
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(256))]
    #[test]
    fn arbitrary_unicode_is_complete_or_a_real_input_error(
        scalars in proptest::collection::vec(proptest::prelude::any::<char>(), 0..100)
    ) {
        let input: String = scalars.into_iter().collect();
        let normalizer = Normalizer::new().unwrap();
        match normalizer.normalize(&input, &options()) {
            Ok(result) => {
                proptest::prop_assert!(result.complete());
                proptest::prop_assert!(result.issues().is_empty());
                proptest::prop_assert_eq!(result.fallback_used(), !result.fallbacks().is_empty());
                proptest::prop_assert_eq!(normalizer.normalize(&input, &options()).unwrap(), result.clone());
                let mut end = 0;
                let mut boundaries: Vec<_> = unicode_segmentation::UnicodeSegmentation::grapheme_indices(input.as_str(), true)
                    .map(|(start, _)| start).collect();
                boundaries.push(input.len());
                for segment in result.segments() {
                    proptest::prop_assert_eq!(segment.range().start(), end);
                    end = segment.range().end();
                    proptest::prop_assert!(boundaries.contains(&end));
                    proptest::prop_assert!(!segment.text().is_empty());
                    proptest::prop_assert_ne!(segment.kind(), SegmentKind::Unresolved);
                    if segment.kind() == SegmentKind::Verbatim {
                        proptest::prop_assert_eq!(segment.text(), &input[segment.range().start()..end]);
                    }
                }
                proptest::prop_assert_eq!(end, input.len());
                proptest::prop_assert_eq!(result.segments().iter().map(|segment| segment.text()).collect::<String>(), result.normalized_text());
                proptest::prop_assert_eq!(result.segments().iter().filter(|segment| segment.kind() == SegmentKind::Fallback).map(|segment| segment.range()).collect::<Vec<_>>(),
                    result.fallbacks().iter().map(|record| record.range()).collect::<Vec<_>>());
            }
            Err(error) => proptest::prop_assert_eq!(error, NormalizeError::InvalidInput),
        }
    }
}
