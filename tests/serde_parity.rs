//! Feature-independent semantics and optional result serialization.
#![cfg(feature = "serde")]

use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, HintKind, IssueCategory,
    NormalizeOptions, Normalizer, SegmentKind,
};

fn labels<T: serde::Serialize>(values: &[T]) -> Vec<serde_json::Value> {
    values
        .iter()
        .map(|value| serde_json::to_value(value).unwrap())
        .collect()
}

#[test]
fn serialized_public_labels_remain_explicit() {
    use SegmentKind::*;
    assert_eq!(
        labels(&[
            Verbatim,
            Unresolved,
            Cardinal,
            Ordinal,
            Decimal,
            Digits,
            Percent,
            Money,
            Unit,
            Date,
            Time,
            Abbreviation,
            Range,
            Telephone,
            Iban,
            Roman,
            Electronic,
            Symbol,
            Fallback,
            Pronunciation,
        ]),
        serde_json::json!([
            "Verbatim",
            "Unresolved",
            "Cardinal",
            "Ordinal",
            "Decimal",
            "Digits",
            "Percent",
            "Money",
            "Unit",
            "Date",
            "Time",
            "Abbreviation",
            "Range",
            "Telephone",
            "Iban",
            "Roman",
            "Electronic",
            "Symbol",
            "Fallback",
            "Pronunciation"
        ])
        .as_array()
        .unwrap()
        .to_vec()
    );
    assert_eq!(
        labels(&[
            AmbiguityPolicy::Preserve,
            AmbiguityPolicy::Reject,
            AmbiguityPolicy::Fallback,
        ]),
        serde_json::json!(["Preserve", "Reject", "Fallback"])
            .as_array()
            .unwrap()
            .to_vec()
    );
    assert_eq!(
        labels(&[
            HintKind::Cardinal,
            HintKind::Digits,
            HintKind::Date,
            HintKind::Time,
            HintKind::Ordinal,
            HintKind::Roman,
            HintKind::Range,
            HintKind::Telephone,
            HintKind::Electronic,
        ]),
        serde_json::json!([
            "Cardinal",
            "Digits",
            "Date",
            "Time",
            "Ordinal",
            "Roman",
            "Range",
            "Telephone",
            "Electronic"
        ])
        .as_array()
        .unwrap()
        .to_vec()
    );
    assert_eq!(
        labels(&[
            IssueCategory::Ambiguous,
            IssueCategory::InvalidExpression,
            IssueCategory::ProtectedIdentifier,
            IssueCategory::Unsupported,
            IssueCategory::UnknownAbbreviation,
        ]),
        serde_json::json!([
            "Ambiguous",
            "InvalidExpression",
            "ProtectedIdentifier",
            "Unsupported",
            "UnknownAbbreviation"
        ])
        .as_array()
        .unwrap()
        .to_vec()
    );
    assert_eq!(
        labels(&[
            FallbackClass::Number,
            FallbackClass::Date,
            FallbackClass::Time,
            FallbackClass::Percent,
            FallbackClass::Quantity,
            FallbackClass::Abbreviation,
            FallbackClass::Identifier,
            FallbackClass::Roman,
            FallbackClass::Electronic,
            FallbackClass::Expression,
            FallbackClass::Symbol,
            FallbackClass::Pronunciation,
        ]),
        serde_json::json!([
            "Number",
            "Date",
            "Time",
            "Percent",
            "Quantity",
            "Abbreviation",
            "Identifier",
            "Roman",
            "Electronic",
            "Expression",
            "Symbol",
            "Pronunciation"
        ])
        .as_array()
        .unwrap()
        .to_vec()
    );
    assert_eq!(
        labels(&[
            FallbackReason::MissingIntent,
            FallbackReason::LeadingZeroes,
            FallbackReason::InvalidForm,
            FallbackReason::ProtectedIdentifier,
            FallbackReason::UnsupportedForm,
            FallbackReason::UnapprovedAbbreviation,
            FallbackReason::UnhandledSymbol,
        ]),
        serde_json::json!([
            "MissingIntent",
            "LeadingZeroes",
            "InvalidForm",
            "ProtectedIdentifier",
            "UnsupportedForm",
            "UnapprovedAbbreviation",
            "UnhandledSymbol"
        ])
        .as_array()
        .unwrap()
        .to_vec()
    );
    assert_eq!(
        labels(&[
            FallbackStrategy::PreferredNumber,
            FallbackStrategy::PreferredDate,
            FallbackStrategy::PreferredTime,
            FallbackStrategy::SurfaceDate,
            FallbackStrategy::SurfaceTime,
            FallbackStrategy::Literal,
            FallbackStrategy::UnicodeCodePoint,
        ]),
        serde_json::json!([
            "PreferredNumber",
            "PreferredDate",
            "PreferredTime",
            "SurfaceDate",
            "SurfaceTime",
            "Literal",
            "UnicodeCodePoint"
        ])
        .as_array()
        .unwrap()
        .to_vec()
    );
}

fn keys(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn serialized_records_keep_their_fields_and_null_category() {
    let n = Normalizer::new().unwrap();
    let result = n
        .normalize(
            "25 TL; AB12; 🙂",
            &NormalizeOptions {
                ambiguity_policy: AmbiguityPolicy::Fallback,
                ..Default::default()
            },
        )
        .unwrap();
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(
        keys(&value),
        [
            "complete",
            "fallback_used",
            "fallbacks",
            "issues",
            "locale",
            "normalized_text",
            "normalizer_id",
            "segments",
        ]
    );
    assert_eq!(
        keys(&value["segments"][0]),
        ["kind", "range", "rule_id", "text"]
    );
    assert_eq!(keys(&value["segments"][0]["range"]), ["end", "start"]);
    assert_eq!(
        keys(&value["fallbacks"][0]),
        [
            "attempted_class",
            "original_category",
            "range",
            "reason",
            "strategy",
        ]
    );
    assert!(
        value["fallbacks"].as_array().unwrap().iter().any(|row| {
            row["original_category"].is_null() && row["reason"] == "UnhandledSymbol"
        })
    );
    let partial = serde_json::to_value(n.normalize("AB12", &Default::default()).unwrap()).unwrap();
    assert_eq!(
        keys(&partial["issues"][0]),
        ["category", "explanation", "range"]
    );
}

#[test]
fn optional_serialization_preserves_readings_and_source_coordinates() {
    let input = include_str!("fixtures/financial-input.txt").trim_end_matches(['\r', '\n']);
    let expected = include_str!("fixtures/financial-output.txt").trim_end_matches(['\r', '\n']);
    let result = Normalizer::new()
        .unwrap()
        .normalize(input, &NormalizeOptions::default())
        .unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["normalized_text"], expected);
    assert_eq!(json["segments"][0]["range"]["start"], 0);
    assert_eq!(json["segments"][0]["range"]["end"], 13);
    assert_eq!(json["issues"], serde_json::json!([]));
    assert_eq!(json["complete"], true);
    let partial = Normalizer::new()
        .unwrap()
        .normalize("25 TL; 1.234", &NormalizeOptions::default())
        .unwrap();
    assert_eq!(serde_json::to_value(partial).unwrap()["complete"], false);
}
