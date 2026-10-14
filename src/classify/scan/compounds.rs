//! Numeric source compounds and external-wrapper certification.
use super::{Token, append_token, punctuation, quotes::QuoteBoundaries};
use crate::{NormalizeError, WorkControl, source_map::SourceMap};

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

/// `quotes` contains paired legacy single-quote endpoints, not an arbitrary offset union.
fn parenthesis_pairs(
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

pub(super) fn append<'a>(
    tokens: &mut Vec<Token<'a>>,
    source: &'a SourceMap,
    raw: &str,
    offset: usize,
    quote_bounds: &QuoteBoundaries,
    parentheses: &mut Option<Vec<ParenthesisPair>>,
    control: &WorkControl,
) -> Result<(), NormalizeError> {
    let quotes = quote_bounds.offsets();
    let mut start = 0;
    let mut depth = 0_usize;
    for (index, ch) in raw.char_indices() {
        control.check()?;
        match ch {
            '(' => depth += 1,
            ')' if depth == 0
                && external_closer(
                    tokens,
                    source,
                    offset,
                    offset + index,
                    quote_bounds.legacy_pairs(),
                    parentheses,
                    control,
                )? =>
            {
                if let Some(range) =
                    punctuation::compound_range(&raw[start..index], offset + start, quotes)
                {
                    append_token(tokens, source, range);
                }
                start = index + ch.len_utf8();
            }
            ')' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => {
                if let Some(range) =
                    punctuation::compound_range(&raw[start..index], offset + start, quotes)
                {
                    append_token(tokens, source, range);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if let Some(range) = punctuation::compound_range(&raw[start..], offset + start, quotes) {
        append_token(tokens, source, range);
    }
    Ok(())
}

fn external_closer(
    tokens: &[Token<'_>],
    source: &SourceMap<'_>,
    raw_start: usize,
    close: usize,
    quotes: &[usize],
    parentheses: &mut Option<Vec<ParenthesisPair>>,
    control: &WorkControl,
) -> Result<bool, NormalizeError> {
    if parentheses.is_none() {
        *parentheses = Some(parenthesis_pairs(source.text(), quotes, control)?);
    }
    let pairs = parentheses.as_ref().ok_or(NormalizeError::Internal)?;
    let Ok(index) = pairs.binary_search_by_key(&close, |pair| pair.close) else {
        return Ok(false);
    };
    let open = pairs[index].open;
    let token = tokens.get(tokens.partition_point(|token| token.range.end <= open));
    Ok(open < raw_start && token.is_none_or(|token| token.range.start > open))
}

#[cfg(test)]
mod tests {
    use super::super::punctuation::quotation_boundaries;
    use super::*;

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
            ("'‘(’' )", vec![]),
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
}
