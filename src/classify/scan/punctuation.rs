//! Source-local quotation, trimming and quantity separator signals.
use crate::domain::lexicon;
use crate::{NormalizeError, SourceRange, WorkControl};

fn delimiter(ch: char) -> bool {
    matches!(
        ch,
        ';' | '!' | '?' | '(' | ')' | '{' | '}' | '[' | ']' | '«' | '»' | '"' | '“' | '”'
    )
}

pub(super) fn quotation_boundaries(text: &str) -> Vec<usize> {
    if !text.contains(['\'', '‘']) {
        return Vec::new();
    }
    let mut boundaries = Vec::new();
    let mut open = None;
    let mut previous = None;
    let mut numeric_base = true;
    let mut has_digit = false;
    let mut numeric_tail = false;
    let mut chars = text.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        let next = chars.peek().map(|(_, ch)| *ch);
        if let Some((start, closing)) = open {
            if ch == closing
                && next.is_none_or(|ch: char| {
                    ch.is_whitespace() || delimiter(ch) || matches!(ch, ',' | '.')
                })
            {
                boundaries.extend([start, offset]);
                open = None;
            }
        } else if matches!(ch, '\'' | '‘')
            && previous.is_none_or(|ch: char| ch.is_whitespace() || delimiter(ch) || ch == ',')
            && next.is_some_and(|ch: char| !ch.is_whitespace())
            // A tail after a numeric closing parenthesis is not a prose quote opener.
            && !(ch == '\''
                && previous == Some(')')
                && ((numeric_base && has_digit) || text[..offset]
                    .chars()
                    .rev()
                    .find(|ch| *ch != ')')
                    .is_some_and(|ch| ch.is_ascii_digit())))
        {
            open = Some((offset, if ch == '‘' { '’' } else { '\'' }));
        }
        if ch.is_whitespace() || ch == ',' || (delimiter(ch) && !matches!(ch, '(' | ')')) {
            numeric_base = true;
            has_digit = false;
            numeric_tail = false;
        } else if !numeric_tail {
            if matches!(ch, '\'' | '’') && numeric_base && has_digit {
                numeric_tail = true;
            } else {
                has_digit |= ch.is_ascii_digit();
                numeric_base &= ch.is_ascii_digit()
                    || matches!(ch, '+' | '-' | '(' | ')')
                    || unicode_normalization::char::is_combining_mark(ch);
            }
        }
        previous = Some(ch);
    }
    boundaries
}

pub(super) fn boundary(ch: char, offset: usize, quotes: &[usize]) -> bool {
    delimiter(ch) || quotes.binary_search(&offset).is_ok()
}

pub(super) struct ParenthesisPair {
    pub(super) open: usize,
    pub(super) close: usize,
}

struct ParenthesisQuoteScope {
    end: usize,
    base: usize,
    smart_double: bool,
    missing_ascii: bool,
    missing_smart: bool,
}

pub(super) fn parenthesis_pairs(
    text: &str,
    quotes: &[usize],
    control: &WorkControl,
) -> Result<Vec<ParenthesisPair>, NormalizeError> {
    let mut openings = Vec::new();
    let mut pairs = Vec::new();
    let mut quote_index = 0;
    let mut scopes: Vec<ParenthesisQuoteScope> = Vec::new();
    let mut smart_double = false;
    let mut missing_ascii = false;
    let mut missing_smart = false;
    for (offset, ch) in text.char_indices() {
        control.check()?;
        let single_event = quotes.get(quote_index) == Some(&offset);
        let single_open = single_event && quote_index.is_multiple_of(2);
        let single_end = if single_open {
            Some(
                *quotes
                    .get(quote_index + 1)
                    .ok_or(NormalizeError::Internal)?,
            )
        } else {
            None
        };
        if single_event {
            quote_index += 1;
        }
        if scopes.last().is_some_and(|scope| scope.end == offset) {
            let scope = scopes.pop().ok_or(NormalizeError::Internal)?;
            openings.truncate(scope.base);
            if scope.smart_double {
                smart_double = false;
            }
            continue;
        }
        let end = scopes.last().map_or(text.len(), |scope| scope.end);
        let mut quote_end = single_end.filter(|single_end| *single_end < end);
        let is_smart = ch == '“';
        if quote_end.is_none() && (ch == '"' || (is_smart && !smart_double)) {
            let missing = if let Some(scope) = scopes.last_mut() {
                if is_smart {
                    &mut scope.missing_smart
                } else {
                    &mut scope.missing_ascii
                }
            } else if is_smart {
                &mut missing_smart
            } else {
                &mut missing_ascii
            };
            if !*missing {
                let closing = if is_smart { '”' } else { '"' };
                quote_end =
                    matching_quote_end(text, offset + ch.len_utf8(), end, closing, control)?;
                *missing = quote_end.is_none();
            }
        }
        if let Some(end) = quote_end {
            scopes.push(ParenthesisQuoteScope {
                end,
                base: openings.len(),
                smart_double: is_smart,
                missing_ascii: false,
                missing_smart: false,
            });
            smart_double |= is_smart;
            continue;
        }
        match ch {
            '(' => openings.push(offset),
            ')' if openings.len() > scopes.last().map_or(0, |scope| scope.base) => {
                pairs.push(ParenthesisPair {
                    open: openings.pop().ok_or(NormalizeError::Internal)?,
                    close: offset,
                });
            }
            _ => {}
        }
    }
    Ok(pairs)
}

fn matching_quote_end(
    text: &str,
    start: usize,
    end: usize,
    closing: char,
    control: &WorkControl,
) -> Result<Option<usize>, NormalizeError> {
    for (offset, ch) in text[start..end].char_indices() {
        control.check()?;
        if ch == closing {
            return Ok(Some(start + offset));
        }
    }
    Ok(None)
}

pub(in crate::classify) fn trimmed_range(
    text: &str,
    offset: usize,
    quotes: &[usize],
) -> Option<SourceRange> {
    trim_range(text, offset, quotes, false)
}

pub(super) fn compound_range(text: &str, offset: usize, quotes: &[usize]) -> Option<SourceRange> {
    trim_range(text, offset, quotes, numeric_parenthesis_compound(text))
}

fn trim_range(
    text: &str,
    offset: usize,
    quotes: &[usize],
    keep_parentheses: bool,
) -> Option<SourceRange> {
    let prose_boundary =
        |ch, offset| boundary(ch, offset, quotes) && !(keep_parentheses && matches!(ch, '(' | ')'));
    let first = text
        .char_indices()
        .find(|(i, ch)| !prose_boundary(*ch, offset + i) && !matches!(ch, ',' | '…'))?
        .0;
    let leading = &text[first..];
    let start = offset + first;
    let mut body = leading;
    while let Some((i, ch)) = body.char_indices().next_back() {
        if !prose_boundary(ch, start + i) && ch != '…' {
            break;
        }
        body = &body[..i];
    }
    if body.ends_with(',') && !body.ends_with(",,") {
        body = &body[..body.len() - 1];
    }
    if let Some(before) = body.strip_suffix(':')
        && !before.contains([':', ','])
        && (lexicon::plain_abbreviation(before) || lexicon::pronunciation_boundary(before))
    {
        body = before;
    }

    if body.ends_with('.')
        && !body.ends_with("..")
        && !lexical_period(body)
        && (!body[..body.len() - 1]
            .bytes()
            .all(|b| b"IVXLCDM".contains(&b))
            || lexicon::plain_abbreviation(&body[..body.len() - 1]))
        && !body[..body.len() - 1].bytes().all(|b| b.is_ascii_digit())
    {
        body = &body[..body.len() - 1];
    }
    while let Some((i, ch)) = body.char_indices().next_back() {
        if !prose_boundary(ch, start + i) {
            break;
        }
        body = &body[..i];
    }
    (!body.is_empty()).then_some(SourceRange::new(start, start + body.len()))
}

pub(super) fn expression_range(
    text: &str,
    offset: usize,
    quotes: &[usize],
    control: &WorkControl,
) -> Result<Option<SourceRange>, NormalizeError> {
    control.check()?;
    if text.starts_with('"') && text.contains('@') {
        let body = text.trim_end_matches([',', ';', '!']);
        return Ok(Some(SourceRange::new(offset, offset + body.len())));
    }
    let Some((first, _)) = text.char_indices().find(|(i, ch)| {
        !matches!(ch, '(' | '[' | '{' | '«' | '"' | '“')
            && quotes.binary_search(&(offset + i)).is_err()
    }) else {
        return Ok(None);
    };
    let leading = &text[first..];
    if crate::domain::electronic::looks_like(leading) {
        let mut body = leading;
        while let Some((i, ch)) = body.char_indices().next_back() {
            control.check()?;
            if !matches!(ch, '.' | ',' | ';' | '!' | '»' | '"' | '”')
                && quotes.binary_search(&(offset + first + i)).is_err()
            {
                break;
            }
            body = &body[..i];
        }
        let mut openings = 0;
        let mut closings = 0;
        for byte in body.bytes() {
            control.check()?;
            openings += usize::from(byte == b'(');
            closings += usize::from(byte == b')');
        }
        while body.ends_with(')') && closings > openings {
            control.check()?;
            body = &body[..body.len() - 1];
            closings -= 1;
        }
        body = body.trim_end_matches([']', '}']);
        let start = offset + text.len() - leading.len();
        return Ok((!body.is_empty()).then_some(SourceRange::new(start, start + body.len())));
    }
    Ok(trimmed_range(text, offset, quotes))
}

pub(super) fn lexical_period(text: &str) -> bool {
    text.ends_with('.') && lexicon::abbreviation(text).is_some()
}

pub(super) fn numeric_parenthesis_compound(raw: &str) -> bool {
    let body = raw
        .strip_prefix('(')
        .and_then(|body| body.strip_suffix(')'))
        .unwrap_or(raw);
    body.contains(['(', ')'])
        && !body.chars().any(char::is_alphabetic)
        && body
            .split(|c: char| !c.is_ascii_digit())
            .filter(|run| !run.is_empty())
            .count()
            > 1
}

pub(super) fn quantity_separators(text: &str, offset: usize, quotes: &[usize]) -> bool {
    let mut start = 0;
    let mut separated = false;
    for (i, ch) in text.char_indices() {
        if matches!(ch, ';' | ',') {
            if crate::notation::quantity_piece(&text[start..i])
                || (ch != ',' && crate::notation::attached_quantity(&text[start..i]).is_some())
            {
                return true;
            }
            separated = true;
            start = i + ch.len_utf8();
        } else if matches!(ch, '"' | '“' | '”') || quotes.binary_search(&(offset + i)).is_ok() {
            start = i + ch.len_utf8();
        }
    }
    separated && crate::notation::quantity_piece(&text[start..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn source_parenthesis_pairs_do_not_escape_certified_quote_scopes() {
        let control = WorkControl::default();
        for (text, expected) in [
            ("'(' (1(2))", vec![(6, 8), (4, 9)]),
            ("\"(\" (1(2))", vec![(6, 8), (4, 9)]),
            ("“(” (1(2))", vec![(10, 12), (8, 13)]),
            ("‘\"(\" (1(2))’", vec![(9, 11), (7, 12)]),
            ("\"'(' (1(2))\"", vec![(7, 9), (5, 10)]),
            ("( 'x)' )", vec![(0, 7)]),
        ] {
            let quotes = quotation_boundaries(text);
            let pairs = parenthesis_pairs(text, &quotes, &control).unwrap();
            assert_eq!(
                pairs
                    .iter()
                    .map(|pair| (pair.open, pair.close))
                    .collect::<Vec<_>>(),
                expected
            );
        }
        let unmatched = "“".repeat(4096);
        assert!(
            parenthesis_pairs(&unmatched, &[], &control)
                .unwrap()
                .is_empty()
        );
        let control = WorkControl::default();
        control.cancel();
        assert!(matches!(
            parenthesis_pairs("(1)", &[], &control),
            Err(NormalizeError::Cancelled)
        ));
    }

    #[test]
    fn electronic_trimming_preserves_balances_and_observes_control() {
        let control = WorkControl::default();
        for (source, expected) in [
            ("(https://ornek.com/(a))", "https://ornek.com/(a)"),
            ("https://ornek.com/(a))", "https://ornek.com/(a)"),
            ("https://ornek.com/(a).", "https://ornek.com/(a)"),
            ("https://ornek.com/)))", "https://ornek.com/"),
            ("https://ornek.com/)(a)", "https://ornek.com/)(a"),
        ] {
            let offset = 5;
            let range = expression_range(source, offset, &[], &control)
                .unwrap()
                .unwrap();
            assert_eq!(&source[range.start - offset..range.end - offset], expected);
        }
        let source = format!("https://ornek.com/{}", ")".repeat(16_000));
        control.cancel();
        assert_eq!(
            expression_range(&source, 0, &[], &control),
            Err(NormalizeError::Cancelled)
        );
        let expired = WorkControl::new(Some(Instant::now() - Duration::from_secs(1)));
        assert_eq!(
            expression_range(&source, 0, &[], &expired),
            Err(NormalizeError::Cancelled)
        );
    }
}
