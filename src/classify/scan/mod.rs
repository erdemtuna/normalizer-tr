//! Tokenization and indexed source traversal.
mod punctuation;
mod signals;

use super::context;
use crate::{
    NormalizeError, SourceRange, WorkControl, resources::Resources, source_map::SourceMap,
};
use punctuation::{
    boundary, expression_range, lexical_list, lexical_period, numeric_parenthesis_compound,
    quantity_separators, quotation_boundaries, trimmed_range,
};
pub(super) use signals::{
    group_whitespace, identifier, math_operator, number_fragment, phones, spaced_compound,
};

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
            || (text[range.start..range.end]
                .trim_end_matches('.')
                .bytes()
                .all(|b| b"IVXLCDM".contains(&b))
                && !crate::domain::lexicon::plain_abbreviation(&text[range.start..range.end]))
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
        let mut approved_list = !protected
            && lexical_list(
                body.split(';').next().unwrap_or(body),
                range.start,
                &quotes,
                text,
                &tokens,
            );
        let list =
            protected && quantity_separators(&text[range.start..range.end], range.start, &quotes);
        if protected && !list {
            append_token(&mut tokens, source, range);
            continue;
        }
        let mut start = 0;
        for (offset, ch) in raw.char_indices() {
            let comma = ch == ','
                && (approved_list
                    || (raw[offset + 1..]
                        .starts_with(|c: char| c.is_ascii_digit() || "+-₺$€£".contains(c))
                        && crate::notation::quantity_piece(&raw[start..offset])));
            if !boundary(ch, matched.start() + offset, &quotes) && !comma {
                continue;
            }
            if list
                && ch != ';'
                && !comma
                && identifier(&raw[start..offset])
                && !crate::notation::quantity_piece(&raw[start..offset])
            {
                continue;
            }
            if let Some(range) =
                trimmed_range(&raw[start..offset], matched.start() + start, &quotes)
            {
                append_token(&mut tokens, source, range);
            }
            start = offset + ch.len_utf8();
            if ch == ';' && !protected {
                let remaining = &raw[start..];
                approved_list = lexical_list(
                    remaining.split(';').next().unwrap_or(remaining),
                    matched.start() + start,
                    &quotes,
                    text,
                    &tokens,
                );
            }
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

fn append_token<'a>(tokens: &mut Vec<Token<'a>>, source: &'a SourceMap, range: SourceRange) {
    let range = source.cover(range);
    tokens.push(Token {
        range,
        text: &source.text()[range.start..range.end],
        number_run_end: 0,
    });
}
