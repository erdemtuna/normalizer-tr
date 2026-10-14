//! Raw atom eligibility; quote framing is supplied by the shared source index.
use super::Token;
use crate::{NormalizeError, WorkControl};

fn left_boundary(
    text: &str,
    mut end: usize,
    quotes: &[bool],
    control: &WorkControl,
) -> Result<bool, NormalizeError> {
    while let Some((offset, ch)) = text[..end].char_indices().next_back() {
        control.check()?;
        if ch.is_whitespace() {
            return Ok(true);
        }
        if !quotes.get(offset).copied().unwrap_or(false) {
            return Ok(false);
        }
        end = offset;
    }
    Ok(true)
}

fn right_boundary(
    text: &str,
    start: usize,
    quotes: &[bool],
    control: &WorkControl,
) -> Result<bool, NormalizeError> {
    for (offset, ch) in text[start..].char_indices() {
        control.check()?;
        if ch.is_whitespace() {
            return Ok(true);
        }
        if !quotes.get(start + offset).copied().unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn mark(
    text: &str,
    tokens: &mut [Token<'_>],
    quotes: &[bool],
    control: &WorkControl,
) -> Result<(), NormalizeError> {
    for token in tokens {
        control.check()?;
        let mut scalars = token.text.chars();
        token.standalone = scalars.next().is_some()
            && scalars.next().is_none()
            && left_boundary(text, token.range.start, quotes, control)?
            && right_boundary(text, token.range.end, quotes, control)?;
    }
    Ok(())
}
