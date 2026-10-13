//! A suffixed phone-like group keeps its whole span as one unsupported owner.
use normalizer_tr::{AmbiguityPolicy, NormalizeError, NormalizeOptions, Normalizer};

fn options(policy: AmbiguityPolicy) -> NormalizeOptions {
    NormalizeOptions {
        ambiguity_policy: policy,
        ..Default::default()
    }
}

#[test]
fn suffixed_phone_is_one_issue_over_its_whole_span() {
    let normalizer = Normalizer::new().unwrap();
    let result = normalizer
        .normalize("0532 123 45 67'den", &options(AmbiguityPolicy::Preserve))
        .unwrap();
    assert!(!result.complete());
    let issues = result.issues();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(
        (issues[0].range().start(), issues[0].range().end()),
        (0, 18)
    );
    assert!(!result.normalized_text().contains("altmış yediden"));
}

#[test]
fn reject_reports_the_same_span_as_preserve() {
    let normalizer = Normalizer::new().unwrap();
    let Err(NormalizeError::Unresolved(issues)) = normalizer.normalize(
        "0532 123 45 67'den; 5 kg",
        &options(AmbiguityPolicy::Reject),
    ) else {
        panic!("reject must stop on the suffixed phone");
    };
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(
        (issues[0].range().start(), issues[0].range().end()),
        (0, 18)
    );
}

#[test]
fn unsuffixed_phone_and_neighbouring_quantity_are_unchanged() {
    let normalizer = Normalizer::new().unwrap();
    let result = normalizer
        .normalize("0532 123 45 67; 5 kg", &options(AmbiguityPolicy::Preserve))
        .unwrap();
    assert!(result.complete(), "{:?}", result.issues());
    assert!(result.normalized_text().ends_with("; beş kilogram"));
}
