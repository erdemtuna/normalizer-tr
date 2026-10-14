//! Written apostrophe suffix decomposition; no spoken-text inference.
pub(crate) fn suffix_parts(text: &str) -> Option<(&str, Vec<&str>)> {
    let mut parts = text.split(['\'', '’']);
    let base = parts.next()?;
    let suffixes: Vec<_> = parts.take(3).collect();
    if suffixes.len() > 2 || suffixes.iter().any(|s| s.is_empty()) {
        return None;
    }
    Some((base, suffixes))
}

pub(crate) fn split_suffix(text: &str) -> Option<(&str, Option<&str>)> {
    let mut parts = text.split(['\'', '’']);
    let base = parts.next()?;
    let suffix = parts.next();
    if parts.next().is_some() || suffix.is_some_and(str::is_empty) {
        return None;
    }
    Some((base, suffix))
}

/// Borrowed source ownership, including malformed tails, not suffix validation.
pub(crate) fn numeric_suffix_surface<'a>(
    text: &'a str,
    control: &crate::WorkControl,
) -> Result<Option<(&'a str, &'a str)>, crate::NormalizeError> {
    let mut apostrophe = None;
    for (offset, ch) in text.char_indices() {
        control.check()?;
        if matches!(ch, '\'' | '’') {
            apostrophe.get_or_insert(offset);
        } else if apostrophe.is_none() {
            if !ch.is_ascii_digit() {
                return Ok(None);
            }
        } else if !ch.is_alphanumeric() {
            return Ok(None);
        }
    }
    Ok(apostrophe
        .filter(|offset| *offset > 0)
        .map(|offset| (&text[..offset], &text[offset..])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NormalizeError, WorkControl};

    #[test]
    fn numeric_source_tails_do_not_certify_grammar_or_extract_other_owners() {
        let control = WorkControl::default();
        for source in ["67'den", "67’den", "67'", "67'den'de", "67'd2"] {
            let (base, tail) = numeric_suffix_surface(source, &control).unwrap().unwrap();
            assert_eq!(base, "67");
            assert_eq!(source, format!("{base}{tail}"));
        }
        for source in [
            "67",
            "'den",
            "67AB12'den",
            "67_12'den",
            "67'den,AB12",
            "67'den,5kg",
            "67'den,Claude",
            "67'den@ornek.com",
            "67'den/5",
            "67'den:5",
            "67'den 5",
        ] {
            assert_eq!(
                numeric_suffix_surface(source, &control),
                Ok(None),
                "{source}"
            );
        }
        assert!(split_suffix("67'").is_none());
        assert!(split_suffix("67'den'de").is_none());
    }

    #[test]
    fn numeric_source_tails_propagate_cancellation() {
        let control = WorkControl::default();
        control.cancel();
        assert_eq!(
            numeric_suffix_surface("67'den", &control),
            Err(NormalizeError::Cancelled)
        );
    }
}
