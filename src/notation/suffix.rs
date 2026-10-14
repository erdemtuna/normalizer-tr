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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TelephoneSurface<'a> {
    pub(crate) base: &'a str,
    pub(crate) closing_parentheses: usize,
}

/// Borrowed telephone source ownership, not value or suffix validation.
pub(crate) fn telephone_suffix_surface<'a>(
    text: &'a str,
    control: &crate::WorkControl,
) -> Result<Option<TelephoneSurface<'a>>, crate::NormalizeError> {
    let mut has_digit = false;
    for (offset, ch) in text.char_indices() {
        control.check()?;
        if matches!(ch, '\'' | '’') {
            if !has_digit {
                return Ok(None);
            }
            let tail = &text[offset..];
            return Ok(
                telephone_tail_surface(tail, control)?.map(|closing_parentheses| {
                    TelephoneSurface {
                        base: &text[..offset],
                        closing_parentheses,
                    }
                }),
            );
        }
        has_digit |= ch.is_ascii_digit();
        if !ch.is_ascii_digit()
            && !matches!(ch, '+' | '-' | '(' | ')')
            && !unicode_normalization::char::is_combining_mark(ch)
        {
            return Ok(None);
        }
    }
    Ok(None)
}

pub(crate) fn telephone_tail_surface(
    text: &str,
    control: &crate::WorkControl,
) -> Result<Option<usize>, crate::NormalizeError> {
    if !text.starts_with(['\'', '’']) {
        return Ok(None);
    }
    let mut closed = false;
    let mut closing_parentheses = 0;
    for ch in text.chars() {
        control.check()?;
        match ch {
            '\'' | '’' => closed = false,
            ')' => {
                closed = true;
                closing_parentheses += 1;
            }
            ch if unicode_normalization::char::is_combining_mark(ch) => {}
            ch if !closed && ch.is_alphanumeric() => {}
            _ => return Ok(None),
        }
    }
    Ok(Some(closing_parentheses))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NormalizeError, WorkControl};

    #[test]
    fn telephone_source_tails_do_not_certify_grammar_or_extract_other_owners() {
        let control = WorkControl::default();
        for source in ["67'den", "67’den", "67'", "67'den'de", "67'd2"] {
            let surface = telephone_suffix_surface(source, &control).unwrap().unwrap();
            assert_eq!(surface.base, "67");
            assert!(source[surface.base.len()..].starts_with(['\'', '’']));
        }
        for source in [
            "45-67'den",
            "45(67)'den",
            "67)'den",
            "67'd\u{308}en",
            "67)\u{301}'den",
        ] {
            let surface = telephone_suffix_surface(source, &control).unwrap().unwrap();
            assert!(source[surface.base.len()..].starts_with(['\'', '’']));
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
            "67'den)AB12",
            "67'den)5kg",
            "67'den)Claude",
        ] {
            assert_eq!(
                telephone_suffix_surface(source, &control),
                Ok(None),
                "{source}"
            );
        }
        assert!(split_suffix("67'").is_none());
        assert!(split_suffix("67'den'de").is_none());
        let surface = telephone_suffix_surface("45(67')'", &control)
            .unwrap()
            .unwrap();
        assert_eq!(surface.closing_parentheses, 1);
        assert_eq!(telephone_tail_surface("'den)'de", &control), Ok(Some(1)));
        assert_eq!(telephone_tail_surface("'den,AB12", &control), Ok(None));
    }

    #[test]
    fn telephone_source_tails_propagate_cancellation() {
        let control = WorkControl::default();
        control.cancel();
        assert_eq!(
            telephone_suffix_surface("67'den", &control),
            Err(NormalizeError::Cancelled)
        );
    }
}
