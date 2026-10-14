//! Raw atom boundaries with independently certified transparent quote pairs.
use super::Token;
use crate::{NormalizeError, WorkControl};

struct OpenQuote {
    start: usize,
    width: usize,
    closing: char,
    kind: usize,
}

fn opening(ch: char) -> Option<(usize, char)> {
    match ch {
        '\'' => Some((0, '\'')),
        '"' => Some((1, '"')),
        '‘' => Some((2, '’')),
        '“' => Some((3, '”')),
        _ => None,
    }
}

pub(super) fn transparent_quotes(
    text: &str,
    control: &WorkControl,
) -> Result<Vec<bool>, NormalizeError> {
    if !text.contains(['\'', '"', '‘', '’', '“', '”']) {
        return Ok(Vec::new());
    }
    let mut transparent = Vec::new();
    let mut stack: Vec<OpenQuote> = Vec::new();
    let mut pending = Vec::new();
    let mut active = [0_usize; 4];
    let mut ambiguous = false;
    let mut previous: Option<char> = None;
    let mut chars = text.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        control.check()?;
        let next = chars.peek().map(|(_, ch)| *ch);
        let parent_closes = stack.len() > 1 && next == Some(stack[stack.len() - 2].closing);
        let closing_boundary = next.is_none_or(char::is_whitespace) || parent_closes;
        let suffix_apostrophe = matches!(ch, '\'' | '’')
            && previous.is_some_and(char::is_alphanumeric)
            && next.is_some_and(char::is_alphanumeric);
        if stack.last().is_some_and(|quote| quote.closing == ch) && !suffix_apostrophe {
            ambiguous |= !closing_boundary;
            let quote = stack.pop().ok_or(NormalizeError::Internal)?;
            active[quote.kind] -= 1;
            pending.push(offset);
            if stack.is_empty() {
                if !ambiguous {
                    if transparent.is_empty() {
                        transparent.resize(text.len(), false);
                    }
                    for position in pending.drain(..) {
                        control.check()?;
                        transparent[position] = true;
                    }
                } else {
                    pending.clear();
                }
                ambiguous = false;
            }
        } else if let Some((kind, closing)) = opening(ch)
            && (previous.is_none_or(char::is_whitespace)
                || stack
                    .last()
                    .is_some_and(|quote| quote.start + quote.width == offset))
            && next.is_some()
        {
            ambiguous |= active[kind] != 0;
            active[kind] += 1;
            stack.push(OpenQuote {
                start: offset,
                width: ch.len_utf8(),
                closing,
                kind,
            });
            pending.push(offset);
        } else if !stack.is_empty() && matches!(ch, '\'' | '"' | '‘' | '’' | '“' | '”') {
            ambiguous |= !suffix_apostrophe;
        }
        previous = Some(ch);
    }
    Ok(transparent)
}

pub(super) fn token_boundaries(
    legacy: Vec<usize>,
    certified: &[bool],
    control: &WorkControl,
) -> Result<Vec<usize>, NormalizeError> {
    if certified.is_empty() {
        return Ok(legacy);
    }
    // Preserve existing quote grammar, adding only certified source boundaries.
    let mut legacy = legacy.into_iter().peekable();
    let mut merged = Vec::new();
    for (offset, transparent) in certified.iter().enumerate() {
        control.check()?;
        if !transparent {
            continue;
        }
        while legacy.peek().is_some_and(|previous| *previous < offset) {
            control.check()?;
            merged.push(legacy.next().ok_or(NormalizeError::Internal)?);
        }
        if legacy.peek() == Some(&offset) {
            legacy.next();
        }
        merged.push(offset);
    }
    for offset in legacy {
        control.check()?;
        merged.push(offset);
    }
    Ok(merged)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_complete_externally_isolated_quote_pairs_are_transparent() {
        for text in ["'S'", "\"S\"", "‘S’", "“S”", "“‘P’ ve ‘S’”"] {
            let quotes = transparent_quotes(text, &WorkControl::default()).unwrap();
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
                transparent_quotes(text, &WorkControl::default())
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
        let quotes = transparent_quotes(text, &WorkControl::default()).unwrap();
        assert!(!quotes[0] && !quotes[2]);
        assert!(quotes[5] && quotes[7]);
    }

    #[test]
    fn certification_preserves_interior_whitespace_and_legacy_boundaries() {
        for text in ["' P'", "'P '", "\"P\t\"", "‘'S'’", "'‘S’'"] {
            let quotes = transparent_quotes(text, &WorkControl::default()).unwrap();
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
            token_boundaries(vec![1, 7, 12], &certified, &WorkControl::default()).unwrap(),
            [0, 1, 7, 10, 12]
        );
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert_eq!(
            token_boundaries(vec![], &certified, &cancelled),
            Err(NormalizeError::Cancelled)
        );
    }
}
