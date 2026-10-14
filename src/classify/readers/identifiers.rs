//! IBAN admission and protected identifier coordination.
mod telephone;

use super::super::scan::{Token, whitespace_between};
use super::{Attempt, Context};
use crate::{
    IssueCategory, NormalizeError, WorkControl,
    domain::{
        identifiers::Iban,
        lexicon,
        quantities::Quantity,
        temporal::{Clock, Date},
    },
    interpretation::Value,
    notation,
};

pub(super) struct IdentifierMatch {
    pub(super) attempt: Attempt,
    pub(super) source_end: usize,
}

pub(super) fn read(
    ctx: &Context<'_>,
    index: usize,
    control: &WorkControl,
) -> Result<Option<IdentifierMatch>, NormalizeError> {
    let text = ctx.text;
    let tokens = ctx.tokens;
    let token = tokens[index];
    let source = token.text;
    let numeric_group_context = tokens
        .get(index + 1)
        .is_some_and(|next| next.text.len() == 4 && next.text.bytes().all(|b| b.is_ascii_digit()))
        && tokens.get(index + 2).is_some_and(|next| {
            next.text.len() == 4 && next.text.bytes().all(|b| b.is_ascii_digit())
        });
    let iban_like = (source.len() >= 4
        && source.as_bytes()[..2].iter().all(u8::is_ascii_alphabetic)
        && source.as_bytes()[2..].iter().all(u8::is_ascii_alphanumeric)
        && (((source.starts_with("TR") || source.starts_with("tr"))
            && (source.as_bytes()[2..].iter().all(u8::is_ascii_digit)
                || source.len() == Iban::CHARACTERS
                || (source.len() == 4
                    && (source.as_bytes()[2..].iter().any(u8::is_ascii_digit)
                        || numeric_group_context))))
            || (source.len() == 4
                && source.as_bytes()[2..].iter().all(u8::is_ascii_digit)
                && numeric_group_context)))
        || (source == "TR"
            && tokens.get(index + 1).is_some_and(|next| {
                next.text.len() == 2 && next.text.bytes().all(|b| b.is_ascii_digit())
            }));
    if iban_like {
        let end = iban_end(text, tokens, index, control)?;
        let whole = &text[token.range.start..tokens[end].range.end];
        return Ok(Some(IdentifierMatch {
            attempt: (
                Iban::parse(whole)
                    .map(Value::Iban)
                    .ok_or(IssueCategory::ProtectedIdentifier),
                end,
            ),
            source_end: tokens[end].range.end,
        }));
    }
    telephone::read(ctx, index, control)
}

fn iban_end(
    text: &str,
    tokens: &[Token<'_>],
    index: usize,
    control: &WorkControl,
) -> Result<usize, NormalizeError> {
    let mut end = index;
    let mut characters = tokens[index].text.len();
    while let Some(next) = tokens.get(end + 1) {
        control.check()?;
        let independent = characters >= Iban::CHARACTERS
            && (Quantity::parse_operand(next.text).is_ok()
                || notation::split_suffix(next.text).is_some_and(|(base, _)| {
                    Date::parse(base).is_some() || Clock::parse(base).is_some()
                }));
        if lexicon::abbreviation(next.text).is_some()
            || notation::label(next.text)
            || independent
            || (characters >= Iban::CHARACTERS
                && next.text.bytes().all(|b| b.is_ascii_digit())
                && tokens.get(end + 2).is_some_and(|label| {
                    notation::label(label.text)
                        && whitespace_between(text, next.range.end, label.range.start)
                }))
        {
            break;
        }
        if !whitespace_between(text, tokens[end].range.end, next.range.start)
            || next.text.is_empty()
            || !(next
                .text
                .bytes()
                .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
                || next.text.starts_with(|ch: char| ch.is_ascii_digit()))
            || (!next.text.bytes().any(|b| b.is_ascii_digit()) && next.text.len() > 4)
            || (characters >= Iban::CHARACTERS
                && !next.text.starts_with(|ch: char| ch.is_ascii_digit()))
        {
            break;
        }
        characters += next.text.len();
        end += 1;
    }
    Ok(end)
}
