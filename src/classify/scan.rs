use crate::{
    NormalizeError, SourceRange, WorkControl, resources::Resources, source_map::SourceMap,
};

use super::context;

#[derive(Clone, Copy)]
pub(super) struct Token<'a> {
    pub(super) range: SourceRange,
    pub(super) text: &'a str,
    pub(super) number_run_end: usize,
}

pub(super) fn overlaps(a: SourceRange, b: SourceRange) -> bool {
    a.start < b.end && b.start < a.end
}

pub(super) fn whitespace_between(text: &str, start: usize, end: usize) -> bool {
    start < end && text[start..end].chars().all(char::is_whitespace)
}
pub(super) fn cue_whitespace(text: &str, start: usize, end: usize) -> bool {
    start == end || whitespace_between(text, start, end)
}

fn delimiter(ch: char) -> bool {
    matches!(
        ch,
        ';' | '!' | '?' | '(' | ')' | '{' | '}' | '[' | ']' | '«' | '»' | '"' | '“' | '”'
    )
}

fn quotation_boundaries(text: &str) -> Vec<usize> {
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

fn boundary(ch: char, offset: usize, quotes: &[usize]) -> bool {
    delimiter(ch) || quotes.binary_search(&offset).is_ok()
}

fn trimmed_range(text: &str, offset: usize, quotes: &[usize]) -> Option<SourceRange> {
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

fn expression_range(text: &str, offset: usize, quotes: &[usize]) -> Option<SourceRange> {
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

fn lexical_period(text: &str) -> bool {
    crate::domain::lexicon::abbreviation(text.split(['\'', '’']).next().unwrap_or(text)).is_some()
}

fn numeric_parenthesis_compound(raw: &str) -> bool {
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

pub(super) fn tokens<'a>(
    source: &'a SourceMap,
    rules: &Resources,
    control: &WorkControl,
) -> Result<Vec<Token<'a>>, NormalizeError> {
    let text = source.text();
    let quotes = quotation_boundaries(text);
    let mut tokens = Vec::new();
    let mut skip_until = 0;
    for matched in rules.tokens.find_iter(text) {
        control.check()?;
        if matched.start() < skip_until {
            continue;
        }
        let raw = matched.as_str();
        if raw.starts_with('"')
            && let Some(address) = rules
                .quoted_email
                .find_at(text, matched.start())
                .filter(|m| m.start() == matched.start())
        {
            let body = address.as_str().trim_end_matches([',', ';', '!']);
            append_token(
                &mut tokens,
                source,
                SourceRange::new(address.start(), address.start() + body.len()),
            );
            skip_until = address.end();
            continue;
        }
        if numeric_parenthesis_compound(raw) {
            append_token(
                &mut tokens,
                source,
                SourceRange::new(matched.start(), matched.end()),
            );
            continue;
        }
        let Some(range) = expression_range(raw, matched.start(), &quotes) else {
            continue;
        };
        if crate::domain::electronic::looks_like(&text[range.start..range.end])
            || lexical_period(&text[range.start..range.end])
            || text[range.start..range.end]
                .trim_end_matches('.')
                .bytes()
                .all(|b| b"IVXLCDM".contains(&b))
        {
            append_token(&mut tokens, source, range);
            continue;
        }
        let body = &text[range.start..range.end];
        let prefix = context::inline_prefix(body).or_else(|| {
            (body.starts_with(':')
                && body[1..].starts_with(|c: char| c.is_ascii_digit())
                && tokens.last().is_some_and(|previous: &Token<'_>| {
                    !previous.text.contains(':')
                        && context::is_cue_word(previous.text)
                        && whitespace_between(text, previous.range.end, range.start)
                }))
            .then_some(1)
        });
        if let Some(prefix) = prefix {
            let body_start = range.start + prefix;
            let body_range = trimmed_range(&text[body_start..matched.end()], body_start, &quotes)
                .ok_or(NormalizeError::Internal)?;
            append_token(
                &mut tokens,
                source,
                SourceRange::new(range.start, range.start + prefix),
            );
            append_token(&mut tokens, source, body_range);
            continue;
        }
        // Identifier punctuation (including URL queries) must not split the token.
        let protected = identifier(&text[range.start..range.end]);
        let list =
            protected && quantity_separators(&text[range.start..range.end], range.start, &quotes);
        if protected && !list {
            append_token(&mut tokens, source, range);
            continue;
        }
        let mut start = 0;
        for (offset, ch) in raw.char_indices() {
            let comma = ch == ','
                && crate::domain::numeric::quantity_piece(&raw[start..offset])
                && raw[offset + 1..]
                    .starts_with(|c: char| c.is_ascii_digit() || "+-₺$€£".contains(c));
            if !boundary(ch, matched.start() + offset, &quotes) && !comma {
                continue;
            }
            if list
                && ch != ';'
                && !comma
                && identifier(&raw[start..offset])
                && !crate::domain::numeric::quantity_piece(&raw[start..offset])
            {
                continue;
            }
            if let Some(range) =
                trimmed_range(&raw[start..offset], matched.start() + start, &quotes)
            {
                append_token(&mut tokens, source, range);
            }
            start = offset + ch.len_utf8();
        }
        if let Some(range) = trimmed_range(&raw[start..], matched.start() + start, &quotes) {
            append_token(&mut tokens, source, range);
        }
    }
    for index in (0..tokens.len()).rev() {
        control.check()?;
        tokens[index].number_run_end = index;
        if let Some(next) = tokens.get(index + 1)
            && number_fragment(tokens[index].text)
            && number_fragment(next.text)
            && group_whitespace(text, tokens[index].range.end, next.range.start)
        {
            tokens[index].number_run_end = next.number_run_end;
        }
    }
    Ok(tokens)
}

fn quantity_separators(text: &str, offset: usize, quotes: &[usize]) -> bool {
    let mut start = 0;
    let mut separated = false;
    for (i, ch) in text.char_indices() {
        if matches!(ch, ';' | ',') {
            if crate::domain::numeric::quantity_piece(&text[start..i])
                || (ch != ','
                    && crate::domain::numeric::attached_quantity(&text[start..i]).is_some())
            {
                return true;
            }
            separated = true;
            start = i + ch.len_utf8();
        } else if matches!(ch, '"' | '“' | '”') || quotes.binary_search(&(offset + i)).is_ok() {
            start = i + ch.len_utf8();
        }
    }
    separated && crate::domain::numeric::quantity_piece(&text[start..])
}

pub(super) fn number_fragment(text: &str) -> bool {
    text.bytes().any(|b| b.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch.is_ascii_digit() || "+-.,–".contains(ch))
}

pub(super) fn group_whitespace(text: &str, start: usize, end: usize) -> bool {
    start < end
        && text[start..end]
            .chars()
            .all(|ch| matches!(ch, ' ' | '\u{a0}' | '\u{202f}'))
}

fn append_token<'a>(tokens: &mut Vec<Token<'a>>, source: &'a SourceMap, range: SourceRange) {
    let range = source.cover(range);
    tokens.push(Token {
        range,
        text: &source.text()[range.start..range.end],
        number_run_end: 0,
    });
}

pub(super) fn phones(
    text: &str,
    tokens: &[Token<'_>],
    rules: &Resources,
    control: &WorkControl,
) -> Result<Vec<SourceRange>, NormalizeError> {
    let mut phones = Vec::new();
    for matched in rules.phone_like.find_iter(text) {
        control.check()?;
        let range = SourceRange::new(matched.start(), matched.end());
        if tokens
            .binary_search_by_key(&range.start, |t| t.range.start)
            .is_ok()
            && tokens
                .binary_search_by_key(&range.end, |t| t.range.end)
                .is_ok()
        {
            phones.push(range);
        }
    }
    Ok(phones)
}

pub(super) fn spaced_compound(text: &str, tokens: &[Token<'_>], index: usize) -> Option<usize> {
    if !tokens[index].text.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut end = index;
    while let (Some(operator), Some(number)) = (tokens.get(end + 1), tokens.get(end + 2)) {
        if !math_operator(operator.text)
            || !number.text.chars().any(|c| c.is_ascii_digit())
            || !whitespace_between(text, tokens[end].range.end, operator.range.start)
            || !whitespace_between(text, operator.range.end, number.range.start)
        {
            break;
        }
        end += 2;
    }
    (end > index).then_some(end)
}

pub(super) fn math_operator(text: &str) -> bool {
    matches!(
        text,
        "/" | "-"
            | "–"
            | "—"
            | ":"
            | "x"
            | "×"
            | "^"
            | "+"
            | "="
            | "*"
            | "÷"
            | "<"
            | ">"
            | "≤"
            | "≥"
            | "≈"
            | "±"
    )
}

pub(super) fn identifier(text: &str) -> bool {
    if text.contains('@') || text.contains("://") || text.starts_with("www.") {
        return true;
    }
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    let letters = base.chars().any(|c| c.is_alphabetic() && !c.is_numeric());
    let digits = base.chars().any(char::is_numeric);
    digits && (letters || base.contains('_'))
}
