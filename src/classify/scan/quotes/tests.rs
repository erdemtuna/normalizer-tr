use super::*;

#[test]
fn only_complete_externally_isolated_quote_pairs_are_transparent() {
    for text in ["'S'", "\"S\"", "‘S’", "“S”", "“‘P’ ve ‘S’”"] {
        let quotes = certify(text, &WorkControl::default()).unwrap();
        for (offset, ch) in text.char_indices() {
            assert_eq!(
                quotes[offset],
                matches!(ch, '\'' | '"' | '‘' | '’' | '“' | '”'),
                "{text}"
            );
        }
    }
    for text in [
        "'S", "\"S", "‘S", "“S", "“S’", "\"S\"X", "X\"S\"", "''S''", "«S»",
    ] {
        assert!(
            certify(text, &WorkControl::default())
                .unwrap()
                .iter()
                .all(|position| !position),
            "{text}"
        );
    }
}

#[test]
fn rejected_external_attachment_does_not_contaminate_later_independent_pairs() {
    let text = "\"S\"X \"P\"";
    let quotes = certify(text, &WorkControl::default()).unwrap();
    assert!(!quotes[0] && !quotes[2]);
    assert!(quotes[5] && quotes[7]);
}

#[test]
fn certification_preserves_interior_whitespace_and_legacy_boundaries() {
    for text in ["' P'", "'P '", "\"P\t\"", "‘'S'’", "'‘S’'"] {
        let quotes = certify(text, &WorkControl::default()).unwrap();
        for (offset, ch) in text.char_indices() {
            assert_eq!(
                quotes[offset],
                matches!(ch, '\'' | '"' | '‘' | '’' | '“' | '”'),
                "{text:?}"
            );
        }
    }
    let mut certified = vec![false; 14];
    for index in [0, 7, 10] {
        certified[index] = true;
    }
    assert_eq!(
        merge(vec![1, 7, 12], &certified, &WorkControl::default()).unwrap(),
        [0, 1, 7, 10, 12]
    );
    let cancelled = WorkControl::default();
    cancelled.cancel();
    assert_eq!(
        merge(vec![], &certified, &cancelled),
        Err(NormalizeError::Cancelled)
    );
}

#[test]
fn collected_views_share_nested_certification_and_keep_no_quote_fast_path() {
    let quotes = QuoteBoundaries::collect("'‘S’'", &WorkControl::default()).unwrap();
    assert_eq!(quotes.offsets(), [0, 1, 5, 8]);
    for offset in quotes.offsets() {
        assert!(quotes.transparent()[*offset]);
    }
    let empty = QuoteBoundaries::collect("S", &WorkControl::default()).unwrap();
    assert!(empty.offsets().is_empty() && empty.transparent().is_empty());
}
