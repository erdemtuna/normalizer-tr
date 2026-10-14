use super::support::{failure, failure_with_strategy};
use normalizer_tr::{FallbackStrategy, Normalizer, SegmentKind, SourceRange};

#[test]
fn isolated_failed_owners_have_full_policy_provenance() {
    let normalizer = Normalizer::new().unwrap();
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../fixtures/policy-contract/telephone-ownership.json"
    ))
    .unwrap();
    for case in cases {
        if case["fallback_class"] != "Identifier" {
            continue;
        }
        let source = case["text"].as_str().unwrap();
        let strategy = if case["strategy"] == "UnicodeCodePoint" {
            FallbackStrategy::UnicodeCodePoint
        } else {
            FallbackStrategy::Literal
        };
        let (preserved, fallback) = failure_with_strategy(
            &normalizer,
            source,
            SourceRange::new(0, source.len()),
            strategy,
        );
        assert_eq!(preserved.normalized_text(), source);
        assert_eq!(
            fallback.normalized_text(),
            case["fallback"].as_str().unwrap()
        );
        assert_eq!(preserved.segments().len(), 1);
        assert_eq!(fallback.segments().len(), 1);
    }
}

#[test]
fn original_prefixes_quotes_and_following_quantities_keep_exact_segments() {
    let normalizer = Normalizer::new().unwrap();
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../fixtures/policy-contract/telephone-ownership.json"
    ))
    .unwrap();
    for case in cases.iter().filter(|case| {
        [
            "phone-suffixed-national-owner",
            "phone-suffixed-mixed-hyphen",
            "phone-suffixed-adjoining-parentheses",
            "phone-suffixed-parenthesized-final-group",
            "phone-suffixed-parenthesized-tail",
            "phone-suffixed-noncomposing-mark",
            "phone-suffixed-marked-closer-before-tail",
            "phone-suffixed-marked-closer-after-tail",
            "phone-suffixed-repeated-after-closer",
            "phone-suffixed-empty-after-closer",
            "phone-suffixed-digit-after-closer",
        ]
        .contains(&case["id"].as_str().unwrap())
    }) {
        let phone = case["text"].as_str().unwrap();
        let literal = case["fallback"].as_str().unwrap();
        let strategy = if case["strategy"] == "UnicodeCodePoint" {
            FallbackStrategy::UnicodeCodePoint
        } else {
            FallbackStrategy::Literal
        };
        for (prefix, closing) in [
            ("", ""),
            ("O\u{308}nce ", ""),
            ("ö\r\n", ""),
            ("tel ", ""),
            ("telefon ", ""),
            ("'", "'"),
            ("‘", "’"),
            ("(", ")"),
            ("O\u{308}nce (", ")"),
        ] {
            let source = format!("{prefix}{phone}{closing}; 5 kg");
            let owner = SourceRange::new(prefix.len(), prefix.len() + phone.len());
            let (preserved, fallback) =
                failure_with_strategy(&normalizer, &source, owner, strategy);
            assert_eq!(
                preserved.normalized_text(),
                format!("{prefix}{phone}{closing}; beş kilogram")
            );
            assert_eq!(
                fallback.normalized_text(),
                format!("{prefix}{literal}{closing}; beş kilogram")
            );
            for (result, kind, rule, text) in [
                (
                    &preserved,
                    SegmentKind::Unresolved,
                    "source.unresolved",
                    phone,
                ),
                (&fallback, SegmentKind::Fallback, "source.fallback", literal),
            ] {
                let mut expected = Vec::new();
                if !prefix.is_empty() {
                    expected.push((
                        0,
                        prefix.len(),
                        SegmentKind::Verbatim,
                        prefix,
                        "source.verbatim",
                    ));
                }
                let quantity = owner.end() + closing.len() + "; ".len();
                expected.extend([
                    (owner.start(), owner.end(), kind, text, rule),
                    (
                        owner.end(),
                        quantity,
                        SegmentKind::Verbatim,
                        &source[owner.end()..quantity],
                        "source.verbatim",
                    ),
                    (
                        quantity,
                        source.len(),
                        SegmentKind::Unit,
                        "beş kilogram",
                        "quantity",
                    ),
                ]);
                let actual: Vec<_> = result
                    .segments()
                    .iter()
                    .map(|s| {
                        (
                            s.range().start(),
                            s.range().end(),
                            s.kind(),
                            s.text(),
                            s.rule_id(),
                        )
                    })
                    .collect();
                assert_eq!(actual, expected);
            }
        }
    }
    let source = "tel 532 123 45 67'den";
    let (preserved, fallback) = failure(&normalizer, source, SourceRange::new(4, source.len()));
    assert_eq!(preserved.normalized_text(), source);
    assert_eq!(
        fallback.normalized_text(),
        "tel beş üç iki bir iki üç dört beş altı yedi kesme de e ne"
    );
}
