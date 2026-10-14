use super::super::scan::{Token, telephone_gap, whitespace_between};
use super::{Attempt, Context};
use crate::{
    IssueCategory, NormalizeError, WorkControl,
    domain::{
        identifiers::{Iban, Telephone},
        lexicon,
        quantities::{NumericRange, Quantity},
        temporal::{Clock, Date},
    },
    interpretation::Value,
    notation,
};
use unicode_segmentation::UnicodeSegmentation;

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
    let zero_group_signal = source == "0"
        && tokens.get(index + 1).is_some_and(|next| {
            next.text.len() == 3 && next.text.bytes().all(|b| b.is_ascii_digit())
        })
        && tokens.get(index + 2).is_some_and(|next| {
            next.text.len() >= 2 && next.text.bytes().all(|b| b.is_ascii_digit())
        });
    let telephone_cue =
        source.bytes().any(|b| b.is_ascii_digit()) && ctx.cue(index, &["telefon", "tel"]);
    let quantity_context = telephone_cue
        && tokens.get(index + 1).is_some_and(|next| {
            notation::label(next.text)
                || ["kişi", "adet", "gün", "yaş"].contains(&lexicon::lookup_key(next.text).as_str())
        });
    let written_prefix = source.split(['-', '(']).next().unwrap_or(source);
    let digits_only = source.bytes().all(|byte| byte.is_ascii_digit());
    let national_prefix = source.starts_with('0')
        && ((digits_only && (source.len() == 4 || source.len() >= 10))
            || (written_prefix.len() == 4
                && written_prefix.bytes().all(|byte| byte.is_ascii_digit()))
            || zero_group_signal
            || telephone_cue);
    if !source.starts_with("+90") && !national_prefix && !source.starts_with("0-") && !telephone_cue
    {
        return Ok(None);
    }
    if source.starts_with("+90")
        || (source.starts_with('0') && source != "0" && source.chars().any(|c| c.is_ascii_digit()))
        || zero_group_signal
        || (source == "0" && ctx.cue(index, &["telefon", "tel"]))
        || (telephone_cue && !quantity_context && source.bytes().all(|b| b.is_ascii_digit()))
        || (source.len() == 10
            && source.bytes().all(|b| b.is_ascii_digit())
            && ctx.cue(index, &["telefon", "tel"]))
    {
        let end = token.phone_run_end.min(tokens.len() - 1);
        let whole = &text[token.range.start..tokens[end].range.end];
        let digits =
            tokens[end].digit_end - index.checked_sub(1).map_or(0, |i| tokens[i].digit_end);
        let phone_cue = telephone_cue;
        let national_shape = national_prefix || (source.starts_with("0-") && digits == 11);
        let contextual_range = tokens
            .get(index + 1)
            .is_some_and(|next| NumericRange::parse(source, Some(next.text)).is_some());
        if !contextual_range
            && !quantity_context
            && ((source.starts_with("+90") && (end > index || digits == 12))
                || national_shape
                || phone_cue)
        {
            let phone = Telephone::parse(whole, phone_cue);
            if phone.is_none() && ctx.money_end(index).is_some() {
                return Ok(None);
            }
            let (end, source_end) = if phone.is_none() {
                telephone_suffix_end(ctx, index, end, control)?
                    .unwrap_or((end, tokens[end].range.end))
            } else {
                (end, tokens[end].range.end)
            };
            return Ok(Some(IdentifierMatch {
                attempt: (
                    phone
                        .map(Value::Telephone)
                        .ok_or(IssueCategory::InvalidExpression),
                    end,
                ),
                source_end,
            }));
        }
    }

    Ok(None)
}

fn telephone_suffix_end(
    ctx: &Context<'_>,
    index: usize,
    end: usize,
    control: &WorkControl,
) -> Result<Option<(usize, usize)>, NormalizeError> {
    let Some(first) = ctx.tokens.get(end + 1) else {
        return Ok(None);
    };
    let gap = &ctx.text[ctx.tokens[end].range.end..first.range.start];
    if !telephone_gap(gap) {
        return Ok(None);
    }
    let mut terminal = end + 1;
    while marked_closer(ctx.tokens[terminal].text, control)? {
        let Some(next) = ctx.tokens.get(terminal + 1) else {
            break;
        };
        let gap = &ctx.text[ctx.tokens[terminal].range.end..next.range.start];
        if gap.chars().any(char::is_whitespace) || !telephone_gap(gap) {
            break;
        }
        terminal += 1;
    }
    let next = ctx.tokens[terminal];
    let start = if terminal > end + 1 {
        if gap.chars().any(char::is_whitespace) {
            return Ok(None);
        }
        ctx.tokens[end].range.start
    } else {
        next.range.start
    };
    let (start, surface) =
        match notation::telephone_suffix_surface(&ctx.text[start..next.range.end], control)? {
            Some(surface) => (start, surface),
            None if start != ctx.tokens[end].range.start
                && !gap.chars().any(char::is_whitespace) =>
            {
                let start = ctx.tokens[end].range.start;
                let Some(surface) =
                    notation::telephone_suffix_surface(&ctx.text[start..next.range.end], control)?
                else {
                    return Ok(None);
                };
                (start, surface)
            }
            None => return Ok(None),
        };
    let mut depth = 0_usize;
    for ch in ctx.text[ctx.tokens[index].range.start..start + surface.base.len()].chars() {
        control.check()?;
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth = depth.saturating_sub(surface.closing_parentheses);
    let mut source_end = next.range.end;
    let mut closed = surface.closing_parentheses > 0;
    loop {
        control.check()?;
        if depth > 0
            && let Some(grapheme) = ctx.text[source_end..].graphemes(true).next()
            && marked_closer(grapheme, control)?
        {
            source_end += grapheme.len();
            depth -= 1;
            closed = true;
            continue;
        }
        while ctx
            .tokens
            .get(terminal + 1)
            .is_some_and(|token| token.range.end <= source_end)
        {
            terminal += 1;
        }
        if closed
            && let Some(token) = ctx.tokens.get(terminal + 1)
            && token.range.start == source_end
            && let Some(closers) = notation::telephone_tail_surface(token.text, control)?
        {
            source_end = token.range.end;
            terminal += 1;
            depth = depth.saturating_sub(closers);
            continue;
        }
        break;
    }
    Ok(Some((terminal, source_end)))
}

fn marked_closer(text: &str, control: &WorkControl) -> Result<bool, NormalizeError> {
    let mut chars = text.chars();
    if chars.next() != Some(')') {
        return Ok(false);
    }
    for ch in chars {
        control.check()?;
        if !unicode_normalization::char::is_combining_mark(ch) {
            return Ok(false);
        }
    }
    Ok(true)
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
