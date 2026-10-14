mod electronic;
mod identifiers;
mod lexical;
mod numeric;
mod products;
mod pronunciation;
mod quantities;

use super::{
    boundaries::{Boundaries, quantity_tail},
    context,
    scan::{self, Token, whitespace_between},
    temporal,
};
use crate::{
    FallbackClass, HintKind, IssueCategory, NormalizeError, WorkControl,
    domain::{
        electronic::Electronic, identifiers::Telephone, numeric::Numeric, quantities::NumericRange,
    },
    interpretation::UnresolvedFinding,
    interpretation::Value,
    notation as numbers,
};

pub(super) type Attempt = (Result<Value, IssueCategory>, usize);

pub(super) struct Context<'a> {
    pub(super) text: &'a str,
    pub(super) tokens: &'a [Token<'a>],
}

impl Context<'_> {
    pub(super) fn number_end(&self, index: usize) -> usize {
        self.tokens[index].number_run_end.min(self.tokens.len() - 1)
    }

    pub(super) fn money_end(&self, index: usize) -> Option<usize> {
        let token = self.tokens[index];
        if !scan::number_fragment(token.text) {
            return None;
        }
        let end = self.number_end(index);
        let next = self.tokens.get(end + 1)?;
        let spaced = end > index
            && numbers::currency_marker(next.text)
            && whitespace_between(self.text, self.tokens[end].range.end, next.range.start);
        let attached =
            scan::group_whitespace(self.text, self.tokens[end].range.end, next.range.start)
                && numbers::attached_money_tail(next.text).is_some();
        (spaced || attached).then_some(end + 1)
    }

    pub(super) fn cue(&self, index: usize, allowed: &[&str]) -> bool {
        index.checked_sub(1).is_some_and(|i| {
            scan::cue_whitespace(
                self.text,
                self.tokens[i].range.end,
                self.tokens[index].range.start,
            ) && allowed.contains(&context::cue_key(self.tokens[i].text).as_str())
        })
    }
}

pub(super) fn hint(text: &str, kind: HintKind, following_ile: bool) -> Option<Value> {
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
            if let Some(range) = temporal::range_hint(text, following_ile) {
                return Some(range);
            }
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
    control: &WorkControl,
) -> Result<Option<(super::Reading, usize)>, NormalizeError> {
    let reading = existing(ctx, index, bounds, control)?;
    let eligible = match &reading {
        None => true,
        Some((super::Reading::Unresolved(finding), end)) => {
            *end == index
                && finding.class() == FallbackClass::Roman
                && finding.category() == Some(IssueCategory::Ambiguous)
                && finding.reason() == crate::FallbackReason::MissingIntent
        }
        _ => false,
    };
    if eligible
        && ctx.tokens[index].standalone
        && let Some(letter) = numbers::LetterName::parse(ctx.tokens[index].text)
    {
        return Ok(Some((
            super::Reading::Resolved(Value::Letter(letter)),
            index,
        )));
    }
    Ok(reading)
}

fn existing(
    ctx: &Context<'_>,
    index: usize,
    bounds: &Boundaries<'_>,
    control: &WorkControl,
) -> Result<Option<(super::Reading, usize)>, NormalizeError> {
    if let Some(attempt) = products::read(ctx, index, control)? {
        return Ok(Some(attempt));
    }
    let annotate = |attempt: Attempt, class| {
        let (reading, end) = attempt;
        let source = &ctx.text[ctx.tokens[index].range.start..ctx.tokens[end].range.end];
        let reading = match reading {
            Ok(value) => super::Reading::Resolved(value),
            Err(category) => {
                super::Reading::Unresolved(UnresolvedFinding::unresolved(source, class, category))
            }
        };
        (reading, end)
    };
    let admit_catalog = |attempt: Attempt| {
        let (value, end) = &attempt;
        if bounds.catalog_allowed(index, *end) {
            return Some(attempt);
        }
        if value.is_err() {
            let source = &ctx.text[ctx.tokens[index].range.start..ctx.tokens[*end].range.end];
            let valid_prefix = source.split_once(',').is_some_and(|(prefix, _)| {
                scan::trimmed_range(prefix, 0, &[]).is_some_and(|range| {
                    crate::domain::lexicon::catalog_form_valid(&prefix[range.start..range.end])
                })
            });
            if !valid_prefix {
                return Some(attempt);
            }
        }
        None
    };
    let priority = electronic::whole(ctx, index)
        .map(|attempt| annotate(attempt, FallbackClass::Electronic))
        .or_else(|| {
            pronunciation::read(ctx, index)
                .and_then(admit_catalog)
                .map(|attempt| annotate(attempt, FallbackClass::Pronunciation))
        })
        .or_else(|| {
            lexical::read(ctx, index)
                .and_then(admit_catalog)
                .map(|attempt| annotate(attempt, FallbackClass::Abbreviation))
        })
        .or_else(|| {
            electronic::contextual(ctx, index)
                .map(|attempt| annotate(attempt, FallbackClass::Electronic))
        });
    if priority.is_some() {
        return Ok(priority);
    }
    if let Some(range) = temporal::range(ctx.text, ctx.tokens, index) {
        return Ok(Some(range));
    }
    let date_shaped = numbers::split_suffix(ctx.tokens[index].text)
        .is_some_and(|(base, _)| crate::domain::temporal::Date::parse(base).is_some());
    if !date_shaped && let Some(attempt) = identifiers::read(ctx, index, control)? {
        return Ok(Some(annotate(attempt, FallbackClass::Identifier)));
    }
    Ok(numeric::read(ctx, index)
        .map(|(attempt, class)| annotate(attempt, class))
        .or_else(|| {
            quantities::read(ctx, index).map(|attempt| annotate(attempt, FallbackClass::Quantity))
        })
        .or_else(|| {
            bounds.phone_at(ctx.tokens[index].range.start).map(|phone| {
                annotate(
                    (
                        Err(IssueCategory::Unsupported),
                        bounds.next_at(phone.end).min(ctx.tokens.len()) - 1,
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
        .or_else(|| token(ctx, index).map(|reading| (reading, index))))
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

fn token(ctx: &Context<'_>, index: usize) -> Option<super::Reading> {
    let token = ctx.tokens[index];
    let unresolved = |class, category| {
        super::Reading::Unresolved(UnresolvedFinding::unresolved(token.text, class, category))
    };
    if scan::identifier(token.text) {
        return Some(unresolved(
            FallbackClass::Identifier,
            IssueCategory::ProtectedIdentifier,
        ));
    }
    if token.text.contains([':', '.', '-', '/']) && token.text.chars().any(char::is_numeric) {
        return temporal::recognize(ctx.text, ctx.tokens, index)
            .map(|(reading, class)| match reading {
                Ok(value) => super::Reading::Resolved(value),
                Err(failure) => super::Reading::Unresolved(UnresolvedFinding::temporal(
                    token.text, class, failure,
                )),
            })
            .or_else(|| Some(automatic_number(token.text)));
    }
    if token.text.starts_with('%') || token.text.chars().any(char::is_numeric) {
        return Some(automatic_number(token.text));
    }
    None
}

fn automatic_number(source: &str) -> super::Reading {
    match Numeric::automatic(source) {
        Ok(value) => super::Reading::Resolved(Value::Numeric(value)),
        Err(failure) => super::Reading::Unresolved(UnresolvedFinding::numeric(source, failure)),
    }
}
