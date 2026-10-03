mod electronic;
mod identifiers;
mod lexical;
mod numeric;
mod quantities;

use super::{
    boundaries::{Boundaries, quantity_tail},
    context,
    scan::{self, Token, whitespace_between},
    temporal,
};
use crate::{
    FallbackClass, HintKind, IssueCategory,
    domain::{
        electronic::Electronic,
        identifiers::Telephone,
        lexicon,
        numeric::{self as numbers, Numeric, NumericRange},
    },
    fallback,
    model::Value,
};

pub(super) type Attempt = (Result<Value, IssueCategory>, usize);

pub(super) struct Context<'a> {
    pub(super) text: &'a str,
    pub(super) tokens: &'a [Token<'a>],
}

impl Context<'_> {
    pub(super) fn cue(&self, index: usize, allowed: &[&str]) -> bool {
        index.checked_sub(1).is_some_and(|i| {
            scan::cue_whitespace(
                self.text,
                self.tokens[i].range.end,
                self.tokens[index].range.start,
            ) && allowed.contains(&context::cue_key(self.tokens[i].text).as_str())
        })
    }
    pub(super) fn contextual_cue_word(&self, index: usize) -> bool {
        let key = context::cue_key(self.tokens[index].text);
        context::is_cue_word(self.tokens[index].text)
            || (["telefon", "tel", "web", "site"].contains(&key.as_str())
                && self.tokens.get(index + 1).is_some_and(|next| {
                    whitespace_between(self.text, self.tokens[index].range.end, next.range.start)
                        && (next.text.contains('.')
                            || next.text.chars().any(|c| c.is_ascii_digit()))
                }))
    }
    /// Anchor roles are recorded once after a successfully read contextual Roman.
    pub(super) fn roman_anchor_end(&self, index: usize) -> Option<usize> {
        let token = self.tokens[index];
        if !token.text.ends_with('.') {
            return None;
        }
        let next = self.tokens.get(index + 1)?;
        if !whitespace_between(self.text, token.range.end, next.range.start) {
            return None;
        }
        match lexicon::lookup_key(next.text).as_str() {
            "yüzyıl" => Some(index + 2),
            "dünya" => self
                .tokens
                .get(index + 2)
                .filter(|last| {
                    lexicon::lookup_key(last.text) == "savaşı"
                        && whitespace_between(self.text, next.range.end, last.range.start)
                })
                .map(|_| index + 3),
            _ => None,
        }
    }
}

pub(super) fn hint(text: &str, kind: HintKind) -> Option<Value> {
    match kind {
        HintKind::Cardinal => Some(Value::Numeric(Numeric::cardinal_hint(text)?)),
        HintKind::Digits => Some(Value::Digits(numbers::digits_hint(text)?)),
        HintKind::Date => temporal::date(text, true).ok(),
        HintKind::Time => temporal::time(text, true).ok(),
        HintKind::Ordinal => Some(Value::Numeric(Numeric::parse(text, true)?)),
        HintKind::Roman => Some(Value::Roman(Numeric::roman(text)?)),
        HintKind::Telephone => Some(Value::Telephone(Telephone::parse(text, true)?)),
        HintKind::Electronic => Some(Value::Electronic(Electronic::parse(text, true)?)),
        HintKind::Range => {
            let (body, noun) = text
                .rsplit_once(' ')
                .map_or((text, None), |(body, noun)| (body, Some(noun)));
            Some(Value::Range(NumericRange::parse(body, noun)?))
        }
    }
}

/// Fixed precedence. Some(Err) seals a matched invalid span; only None continues.
pub(super) fn read(
    ctx: &Context<'_>,
    index: usize,
    bounds: &Boundaries<'_>,
    contextual_role: bool,
) -> Option<(super::Reading, usize)> {
    let annotate = |attempt: Attempt, class| {
        let (reading, end) = attempt;
        let source = &ctx.text[ctx.tokens[index].range.start..ctx.tokens[end].range.end];
        let reading = match reading {
            Ok(value) => super::Reading::Resolved(value),
            Err(category) => {
                super::Reading::Unresolved(fallback::Request::unresolved(source, class, category))
            }
        };
        (reading, end)
    };
    electronic::whole(ctx, index)
        .map(|attempt| annotate(attempt, FallbackClass::Electronic))
        .or_else(|| {
            lexical::read(ctx, index).map(|attempt| annotate(attempt, FallbackClass::Abbreviation))
        })
        .or_else(|| {
            electronic::contextual(ctx, index)
                .map(|attempt| annotate(attempt, FallbackClass::Electronic))
        })
        .or_else(|| {
            identifiers::read(ctx, index)
                .map(|attempt| annotate(attempt, FallbackClass::Identifier))
        })
        .or_else(|| numeric::read(ctx, index).map(|(attempt, class)| annotate(attempt, class)))
        .or_else(|| {
            quantities::read(ctx, index).map(|attempt| annotate(attempt, FallbackClass::Quantity))
        })
        .or_else(|| {
            bounds.phone_at(ctx.tokens[index].range.start).map(|phone| {
                annotate(
                    (
                        Err(IssueCategory::Unsupported),
                        bounds.next_at(phone.end) - 1,
                    ),
                    FallbackClass::Identifier,
                )
            })
        })
        .or_else(|| {
            scan::spaced_compound(ctx.text, ctx.tokens, index).map(|end| {
                annotate(
                    (Err(IssueCategory::Unsupported), end),
                    FallbackClass::Expression,
                )
            })
        })
        .or_else(|| {
            unsupported_quantity(ctx, index)
                .map(|attempt| annotate(attempt, FallbackClass::Quantity))
        })
        .or_else(|| token(ctx, index, contextual_role).map(|reading| (reading, index)))
}

fn unsupported_quantity(ctx: &Context<'_>, index: usize) -> Option<Attempt> {
    let token = ctx.tokens[index];
    let text = ctx.text;
    let tokens = ctx.tokens;
    let next = tokens.get(index + 1);
    if matches!(token.text, "%" | "+" | "-" | "√" | "∛")
        && next.is_some_and(|t| {
            t.text.chars().any(char::is_numeric)
                && whitespace_between(text, token.range.end, t.range.start)
        })
    {
        return Some((Err(IssueCategory::Unsupported), index + 1));
    }
    if numbers::unsupported_label(token.text)
        && next.is_some_and(|t| {
            t.text.chars().any(|c| c.is_ascii_digit())
                && whitespace_between(text, token.range.end, t.range.start)
        })
    {
        return Some((Err(IssueCategory::Unsupported), index + 1));
    }
    if token.text.starts_with('¥') || token.text.ends_with('¥') {
        return Some((Err(IssueCategory::Unsupported), index));
    }
    if token.text.chars().any(|c| c.is_ascii_digit())
        && next.is_some_and(|t| {
            quantity_tail(t.text) && whitespace_between(text, token.range.end, t.range.start)
        })
    {
        return Some((Err(IssueCategory::Unsupported), index + 1));
    }
    None
}

fn token(ctx: &Context<'_>, index: usize, contextual_role: bool) -> Option<super::Reading> {
    let token = ctx.tokens[index];
    let unresolved = |class, category| {
        super::Reading::Unresolved(fallback::Request::unresolved(token.text, class, category))
    };
    if scan::identifier(token.text) {
        return Some(unresolved(
            FallbackClass::Identifier,
            IssueCategory::ProtectedIdentifier,
        ));
    }
    if token.text.contains([':', '.', '-']) && token.text.chars().any(char::is_numeric) {
        return temporal::recognize(ctx.text, ctx.tokens, index)
            .map(|(reading, class)| match reading {
                Ok(value) => super::Reading::Resolved(value),
                Err(failure) => super::Reading::Unresolved(fallback::Request::temporal(
                    token.text, class, failure,
                )),
            })
            .or_else(|| Some(automatic_number(token.text)));
    }
    if token.text.starts_with('%') || token.text.chars().any(char::is_numeric) {
        return Some(automatic_number(token.text));
    }
    (scan::unknown_abbreviation(token.text) && !contextual_role && !ctx.contextual_cue_word(index))
        .then(|| {
            unresolved(
                FallbackClass::Abbreviation,
                IssueCategory::UnknownAbbreviation,
            )
        })
}

fn automatic_number(source: &str) -> super::Reading {
    match Numeric::automatic(source) {
        Ok(value) => super::Reading::Resolved(Value::Numeric(value)),
        Err(failure) => super::Reading::Unresolved(fallback::Request::numeric(source, failure)),
    }
}
