//! Tokenization and indexed source traversal.
mod lists;
mod piece;
mod punctuation;
mod quotes;
mod signals;
mod standalone;

use super::context;
use crate::{
    NormalizeError, SourceRange, WorkControl, resources::Resources, source_map::SourceMap,
};
pub(super) use lists::CatalogGroup;
use piece::Piece;
pub(in crate::classify) use punctuation::trimmed_range;
use punctuation::{
    boundary, expression_range, lexical_period, numeric_parenthesis_compound, quantity_separators,
};
use quotes::QuoteBoundaries;
pub(super) use signals::{
    group_whitespace, identifier, math_operator, number_fragment, phones, spaced_compound,
};

#[derive(Clone, Copy)]
pub(super) struct Token<'a> {
    pub(super) range: SourceRange,
    pub(super) text: &'a str,
    pub(super) number_run_end: usize,
    pub(super) phone_run_end: usize,
    pub(super) digit_end: usize,
    pub(super) standalone: bool,
}

pub(super) struct Scanned<'a> {
    pub(super) tokens: Vec<Token<'a>>,
    pub(super) catalog_groups: Vec<CatalogGroup>,
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
) -> Result<Scanned<'a>, NormalizeError> {
    let text = source.text();
    let quote_bounds = QuoteBoundaries::collect(text, control)?;
    let quotes = quote_bounds.offsets();
    let catalog_groups = lists::groups(text, rules, quotes, control)?;
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
        let Some(range) = expression_range(raw, matched.start(), quotes, control)? else {
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
            let body_range = trimmed_range(&text[body_start..matched.end()], body_start, quotes)
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
        let mut approved_list = approved(&catalog_groups, range.start);
        let list =
            protected && quantity_separators(&text[range.start..range.end], range.start, quotes);
        if protected && !list {
            append_token(&mut tokens, source, range);
            continue;
        }
        let mut start = 0;
        let mut piece = Piece::default();
        for (offset, ch) in raw.char_indices() {
            control.check()?;
            let preceding = &raw[start..offset];
            let delimiter = boundary(ch, matched.start() + offset, quotes);
            let quantity = (delimiter || ch == ',') && piece.quantity(preceding);
            let comma = ch == ','
                && (approved_list
                    || (raw[offset + 1..]
                        .starts_with(|c: char| c.is_ascii_digit() || "+-₺$€£".contains(c))
                        && quantity));
            if !delimiter && !comma {
                piece.advance(preceding, ch);
                continue;
            }
            if list && ch != ';' && !comma && piece.identifier(preceding) && !quantity {
                piece.advance(preceding, ch);
                continue;
            }
            if let Some(range) = trimmed_range(&raw[start..offset], matched.start() + start, quotes)
            {
                append_token(&mut tokens, source, range);
            }
            start = offset + ch.len_utf8();
            piece = Piece::default();
            if ch == ';' {
                approved_list = approved(&catalog_groups, matched.start() + start);
            }
        }
        if let Some(range) = trimmed_range(&raw[start..], matched.start() + start, quotes) {
            append_token(&mut tokens, source, range);
        }
    }
    account_periods(text, &mut tokens, control)?;
    standalone::mark(text, &mut tokens, quote_bounds.transparent(), control)?;
    let mut digits = 0;
    for token in &mut tokens {
        control.check()?;
        digits += token.text.bytes().filter(u8::is_ascii_digit).count();
        token.digit_end = digits;
    }
    for index in (0..tokens.len()).rev() {
        control.check()?;
        tokens[index].number_run_end = index;
        tokens[index].phone_run_end = index;
        if let Some(next) = tokens.get(index + 1)
            && number_fragment(tokens[index].text)
            && number_fragment(next.text)
            && group_whitespace(text, tokens[index].range.end, next.range.start)
        {
            tokens[index].number_run_end = next.number_run_end;
        }
        if let Some(next) = tokens.get(index + 1)
            && signals::telephone_group(next.text)
            && text[tokens[index].range.end..next.range.start]
                .chars()
                .all(|ch| ch.is_whitespace() || matches!(ch, '(' | ')' | '-'))
        {
            tokens[index].phone_run_end = next.phone_run_end;
        }
    }
    Ok(Scanned {
        tokens,
        catalog_groups,
    })
}

fn account_periods(
    text: &str,
    tokens: &mut [Token<'_>],
    control: &WorkControl,
) -> Result<(), NormalizeError> {
    let mut account = false;
    let mut previous_end = 0;
    for token in tokens {
        control.check()?;
        let adjacent =
            previous_end == 0 || whitespace_between(text, previous_end, token.range.start);
        let start = token.text.len() >= 4
            && (token.text.starts_with("TR") || token.text.starts_with("tr"))
            && token.text.as_bytes()[2..].iter().all(u8::is_ascii_digit);
        if start || token.text == "TR" {
            account = true;
        } else if account && adjacent {
            if let Some(body) = token.text.strip_suffix('.')
                && !body.is_empty()
                && body.bytes().all(|byte| byte.is_ascii_digit())
            {
                token.text = body;
                token.range.end -= 1;
                account = false;
            } else if !token
                .text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
            {
                account = false;
            }
        } else {
            account = false;
        }
        previous_end = token.range.end;
    }
    Ok(())
}

fn approved(groups: &[CatalogGroup], offset: usize) -> bool {
    let index = groups.partition_point(|group| group.range.end <= offset);
    groups
        .get(index)
        .is_some_and(|group| group.range.start <= offset && group.approved)
}

fn append_token<'a>(tokens: &mut Vec<Token<'a>>, source: &'a SourceMap, range: SourceRange) {
    let range = source.cover(range);
    tokens.push(Token {
        range,
        text: &source.text()[range.start..range.end],
        number_run_end: 0,
        phone_run_end: 0,
        digit_end: 0,
        standalone: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_phone_groups_match_local_traversal_and_prefix_digit_counts() {
        let rules = Resources::load().unwrap();
        let control = WorkControl::default();
        for text in [
            "01 1 01 1",
            "+90 555 123 45 67",
            "0850 (222) 33-44",
            "telefon 555\t123\n45 67",
            "01 1a 12345b 3 4",
            "0 555 123 45 67; 5 kg",
            "TR33 0006 1005 1978 6457 8413 26.",
        ] {
            let source = SourceMap::new(text, &control).unwrap();
            let scanned = tokens(&source, &rules, &control).unwrap();
            let tokens = scanned.tokens;
            for index in 0..tokens.len() {
                let mut end = index;
                while let Some(next) = tokens.get(end + 1) {
                    if !signals::telephone_group(next.text)
                        || !text[tokens[end].range.end..next.range.start]
                            .chars()
                            .all(|ch| ch.is_whitespace() || matches!(ch, '(' | ')' | '-'))
                    {
                        break;
                    }
                    end += 1;
                }
                assert_eq!(tokens[index].phone_run_end, end, "{text}");
                for limit in index..=end {
                    let indexed = tokens[limit].digit_end
                        - index.checked_sub(1).map_or(0, |i| tokens[i].digit_end);
                    let actual = text[tokens[index].range.start..tokens[limit].range.end]
                        .bytes()
                        .filter(u8::is_ascii_digit)
                        .count();
                    assert_eq!(indexed, actual, "{text}");
                }
            }
        }
    }
}
