//! Source-local quotation, trimming and quantity-list boundaries.
use crate::SourceRange;

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
        {
            open = Some((offset, if ch == '‘' { '’' } else { '\'' }));
        }
        previous = Some(ch);
    }
    boundaries
}

pub(super) fn boundary(ch: char, offset: usize, quotes: &[usize]) -> bool {
    delimiter(ch) || quotes.binary_search(&offset).is_ok()
}

pub(super) fn trimmed_range(text: &str, offset: usize, quotes: &[usize]) -> Option<SourceRange> {
    let first = text
        .char_indices()
        .find(|(i, ch)| !boundary(*ch, offset + i, quotes) && !matches!(ch, ',' | '…'))?
        .0;
    let leading = &text[first..];
    let start = offset + first;
    let mut body = leading;
    while let Some((i, ch)) = body.char_indices().next_back() {
        if !boundary(ch, start + i, quotes) && ch != '…' {
            break;
        }
        body = &body[..i];
    }
    if body.ends_with(',') && !body.ends_with(",,") {
        body = &body[..body.len() - 1];
    }

    if body.ends_with('.')
        && !body.ends_with("..")
        && !lexical_period(body)
        && !body[..body.len() - 1]
            .bytes()
            .all(|b| b"IVXLCDM".contains(&b))
        && !body[..body.len() - 1].bytes().all(|b| b.is_ascii_digit())
    {
        body = &body[..body.len() - 1];
    }
    while let Some((i, ch)) = body.char_indices().next_back() {
        if !boundary(ch, start + i, quotes) {
            break;
        }
        body = &body[..i];
    }
    (!body.is_empty()).then_some(SourceRange::new(start, start + body.len()))
}

pub(super) fn expression_range(text: &str, offset: usize, quotes: &[usize]) -> Option<SourceRange> {
    if text.starts_with('"') && text.contains('@') {
        let body = text.trim_end_matches([',', ';', '!']);
        return Some(SourceRange::new(offset, offset + body.len()));
    }
    let first = text
        .char_indices()
        .find(|(i, ch)| {
            !matches!(ch, '(' | '[' | '{' | '«' | '"' | '“')
                && quotes.binary_search(&(offset + i)).is_err()
        })?
        .0;
    let leading = &text[first..];
    if crate::domain::electronic::looks_like(leading) {
        let mut body = leading;
        while let Some((i, ch)) = body.char_indices().next_back() {
            if !matches!(ch, '.' | ',' | ';' | '!' | '»' | '"' | '”')
                && quotes.binary_search(&(offset + first + i)).is_err()
            {
                break;
            }
            body = &body[..i];
        }
        while body.ends_with(')') && body.matches(')').count() > body.matches('(').count() {
            body = &body[..body.len() - 1];
        }
        body = body.trim_end_matches([']', '}']);
        let start = offset + text.len() - leading.len();
        return (!body.is_empty()).then_some(SourceRange::new(start, start + body.len()));
    }
    trimmed_range(text, offset, quotes)
}

pub(super) fn lexical_period(text: &str) -> bool {
    crate::domain::lexicon::abbreviation(text.split(['\'', '’']).next().unwrap_or(text)).is_some()
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
