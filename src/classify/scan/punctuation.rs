//! Source-local quotation, trimming and approved-list boundaries.
use super::Token;
use crate::SourceRange;
use crate::domain::lexicon;

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
    text.ends_with('.') && lexicon::abbreviation(text).is_some()
}

pub(super) fn lexical_list(
    text: &str,
    offset: usize,
    quotes: &[usize],
    source: &str,
    previous: &[Token<'_>],
) -> bool {
    text.contains(',')
        && text
            .split(',')
            .scan(offset, |start, member| {
                let member_start = *start;
                *start += member.len() + 1;
                Some(
                    trimmed_range(member, member_start, quotes).is_some_and(|range| {
                        let text = &member[range.start - member_start..range.end - member_start];
                        lexicon::plain_abbreviation(text)
                            || pronunciation_member(source, range, previous)
                    }),
                )
            })
            .all(|approved| approved)
}

fn pronunciation_member(source: &str, range: SourceRange, previous: &[Token<'_>]) -> bool {
    let member = &source[range.start..range.end];
    if lexicon::pronunciation_word(member) {
        return true;
    }
    let base = member.split(['\'', '’']).next().unwrap_or(member);
    let base_end = range.start + base.len();
    for token in previous
        .iter()
        .rev()
        .take(lexicon::MAX_PRONUNCIATION_WORDS - 1)
    {
        let phrase = &source[token.range.start..base_end];
        if !phrase.contains(['\'', '’']) && lexicon::pronunciation_word(phrase) {
            return true;
        }
    }
    if base != member {
        return false;
    }
    lexicon::pronunciation_candidates(base)
        .iter()
        .any(|(key, _)| {
            if !key.contains(' ') || source.get(range.start..range.start + key.len()) != Some(*key)
            {
                return false;
            }
            let last_start = range.start + key.len() - key.rsplit(' ').next().unwrap().len();
            let endpoint = source[last_start..].split_whitespace().next().unwrap();
            if crate::domain::electronic::looks_like(endpoint) {
                return false;
            }
            let remainder = &source[range.start + key.len()..];
            let mut chars = remainder.chars();
            match chars.next() {
                None => true,
                Some(ch) if ch.is_whitespace() || matches!(ch, ',' | ';' | '\'' | '’') => true,
                Some('.' | ':' | '!' | '?' | ')' | ']' | '}' | '"' | '”') => chars
                    .next()
                    .is_none_or(|ch| ch.is_whitespace() || matches!(ch, ',' | ';')),
                _ => false,
            }
        })
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
