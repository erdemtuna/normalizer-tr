//! Protected/source-shaped recognition signals, without value rendering.
use super::Token;
use super::whitespace_between;
use crate::{NormalizeError, SourceRange, WorkControl, resources::Resources};

pub(in crate::classify) fn number_fragment(text: &str) -> bool {
    text.bytes().any(|b| b.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch.is_ascii_digit() || "+-.,–".contains(ch))
}

pub(in crate::classify) fn group_whitespace(text: &str, start: usize, end: usize) -> bool {
    start < end
        && text[start..end]
            .chars()
            .all(|ch| matches!(ch, ' ' | '\u{a0}' | '\u{202f}'))
}

pub(in crate::classify) fn phones(
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

pub(in crate::classify) fn spaced_compound(
    text: &str,
    tokens: &[Token<'_>],
    index: usize,
) -> Option<usize> {
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

pub(in crate::classify) fn math_operator(text: &str) -> bool {
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

pub(in crate::classify) fn identifier(text: &str) -> bool {
    if text.contains('@') || text.contains("://") || text.starts_with("www.") {
        return true;
    }
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    let letters = base.chars().any(|c| c.is_alphabetic() && !c.is_numeric());
    let digits = base.chars().any(char::is_numeric);
    digits && (letters || base.contains('_'))
}
