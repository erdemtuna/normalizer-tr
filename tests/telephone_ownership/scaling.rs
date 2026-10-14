use super::support::{LITERAL, PHONE, POLICIES, failure, options, partition};
use normalizer_tr::{
    AmbiguityPolicy, LimitKind, MAX_INPUT_BYTES, NormalizeError, Normalizer, SourceRange,
};

#[test]
fn maximal_marked_closer_remains_bounded_before_address_fields() {
    let normalizer = Normalizer::new().unwrap();
    for count in [64, 16000] {
        let source = format!(
            "adres No:5 0532 123 45 (67'den){} Kat:3",
            "\u{308}".repeat(count)
        );
        assert!(source.len() <= MAX_INPUT_BYTES);
        let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
        partition(&source, &preserved);
        let owner = SourceRange::new("adres No:5 ".len(), source.find(" Kat:3").unwrap());
        assert_eq!(preserved.issues().len(), 1);
        assert_eq!(preserved.issues()[0].range(), owner);
        assert!(preserved.normalized_text().ends_with(" kat üç"));
        assert_eq!(
            normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
        );
        match normalizer.normalize(&source, &options(AmbiguityPolicy::Fallback)) {
            Ok(result) => {
                partition(&source, &result);
                assert!(result.complete() && result.issues().is_empty());
                assert_eq!(result.fallbacks().len(), 1);
                assert_eq!(result.fallbacks()[0].range(), owner);
                assert!(result.normalized_text().ends_with(" kat üç"));
            }
            Err(NormalizeError::LimitExceeded(LimitKind::Result)) => assert_eq!(count, 16000),
            other => panic!("unexpected bounded marked-closer outcome: {other:?}"),
        }
    }
}

#[test]
fn alternating_closers_and_tails_remain_one_bounded_owner() {
    let normalizer = Normalizer::new().unwrap();
    for count in [1, 16, 128, 512] {
        let source = format!(
            "0532 123 {}67'den{}",
            "(".repeat(count),
            ")'de".repeat(count)
        );
        let (preserved, fallback) =
            failure(&normalizer, &source, SourceRange::new(0, source.len()));
        assert_eq!(preserved.normalized_text(), source);
        assert_eq!(fallback.fallbacks().len(), 1);
    }
}

#[test]
fn maximal_alternating_tail_chain_keeps_its_owner_or_reports_a_real_result_limit() {
    let normalizer = Normalizer::new().unwrap();
    let prefix = "0532 123 ";
    let middle = "67'den";
    let opening = "(";
    let closing_tail = ")'de";
    let count =
        (MAX_INPUT_BYTES - prefix.len() - middle.len()) / (opening.len() + closing_tail.len());
    let source = format!(
        "{prefix}{}{middle}{}",
        opening.repeat(count),
        closing_tail.repeat(count)
    );
    let owner = SourceRange::new(0, source.len());
    let preserved = normalizer.normalize(&source, &Default::default()).unwrap();
    assert_eq!(preserved.normalized_text(), source);
    assert_eq!(preserved.issues().len(), 1);
    assert_eq!(preserved.issues()[0].range(), owner);
    assert_eq!(
        normalizer.normalize(&source, &options(AmbiguityPolicy::Reject)),
        Err(NormalizeError::Unresolved(preserved.issues().to_vec()))
    );
    match normalizer.normalize(&source, &options(AmbiguityPolicy::Fallback)) {
        Ok(result) => {
            partition(&source, &result);
            assert!(result.complete() && result.issues().is_empty());
            assert_eq!(result.fallbacks().len(), 1);
            assert_eq!(result.fallbacks()[0].range(), owner);
        }
        Err(NormalizeError::LimitExceeded(LimitKind::Result)) => {}
        other => panic!("unexpected maximal tail-chain outcome: {other:?}"),
    }
}

#[test]
fn long_malformed_owners_preserve_work_and_result_budgets() {
    let normalizer = Normalizer::new().unwrap();
    let stem = "0532 123 45 67'";
    for size in [1024, 4096, 8192, 16384, MAX_INPUT_BYTES] {
        let source = format!("{stem}{}", "d".repeat(size - stem.len()));
        let (preserved, fallback) = failure(&normalizer, &source, SourceRange::new(0, size));
        assert_eq!(preserved.normalized_text(), source);
        assert_eq!(fallback.segments().len(), 1);
        assert_eq!(
            fallback.normalized_text(),
            format!(
                "sıfır beş üç iki bir iki üç dört beş altı yedi kesme{}",
                " de".repeat(size - stem.len())
            )
        );
    }
    for count in [1, 64, 256, 512] {
        let source = format!("{PHONE}; ").repeat(count);
        for policy in POLICIES {
            let result = normalizer.normalize(&source, &options(policy));
            if policy == AmbiguityPolicy::Reject {
                let Err(NormalizeError::Unresolved(issues)) = result else {
                    panic!("expected unresolved repeated phones");
                };
                assert_eq!(issues.len(), count);
                continue;
            }
            let result = result.unwrap();
            let ranges: Vec<_> = if policy == AmbiguityPolicy::Fallback {
                assert!(result.complete() && result.issues().is_empty());
                assert_eq!(
                    result.normalized_text(),
                    format!("{LITERAL}; ").repeat(count)
                );
                result.fallbacks().iter().map(|f| f.range()).collect()
            } else {
                assert_eq!(result.normalized_text(), source);
                result.issues().iter().map(|i| i.range()).collect()
            };
            assert_eq!(
                ranges,
                (0..count)
                    .map(|i| {
                        SourceRange::new(i * (PHONE.len() + 2), i * (PHONE.len() + 2) + PHONE.len())
                    })
                    .collect::<Vec<_>>()
            );
            partition(&source, &result);
        }
    }
    let amplified = format!("{stem}{}", "w".repeat(MAX_INPUT_BYTES - stem.len()));
    assert!(matches!(
        normalizer.normalize(&amplified, &options(AmbiguityPolicy::Fallback)),
        Err(NormalizeError::LimitExceeded(LimitKind::Result))
    ));
}
