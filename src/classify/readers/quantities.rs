use super::super::{
    boundaries::quantity_tail,
    scan::{Token, group_whitespace, number_fragment, whitespace_between},
};
use super::{Attempt, Context};
use crate::domain::numeric::split_suffix;
use crate::{
    IssueCategory,
    domain::{
        lexicon::{self, Currency},
        numeric::{self, NumericRange, Quantity},
    },
    model::Value,
};
pub(super) fn read(ctx: &Context<'_>, index: usize) -> Option<Attempt> {
    let text = ctx.text;
    let tokens = ctx.tokens;
    let token = tokens[index];
    let source = token.text;
    if let Some(attempt) = contextual_range(ctx, index) {
        return Some(attempt);
    }
    let next = tokens
        .get(index + 1)
        .filter(|next| whitespace_between(text, token.range.end, next.range.start));
    if numeric::label(source)
        && let Some(next) = next.filter(|next| next.text.chars().any(|c| c.is_ascii_digit()))
    {
        let base = source.split(['\'', '’']).next().unwrap_or(source);
        let end = if Currency::parse(base).is_some() {
            amount_end(ctx, index + 1)
        } else {
            index + 1
        };
        let number = &text[next.range.start..tokens[end].range.end];
        return Some(finish(
            ctx,
            end,
            prefixed_quantity(number, source)
                .map(Value::Quantity)
                .ok_or(IssueCategory::InvalidExpression),
        ));
    }
    if let Some(end) = ctx.money_end(index) {
        let label = tokens[end];
        let base = label.text.split(['\'', '’']).next().unwrap_or(label.text);
        if let Some((last, currency)) = numeric::attached_quantity(label.text) {
            if Currency::parse(currency).is_none() {
                return Some(finish(ctx, end, Err(IssueCategory::Unsupported)));
            }
            let number = &text[token.range.start..label.range.start + last.len()];
            let quantity = split_suffix(label.text)
                .and_then(|(_, suffix)| Quantity::parse_with_suffix(number, currency, suffix));
            return Some(finish(
                ctx,
                end,
                quantity
                    .map(Value::Quantity)
                    .ok_or(IssueCategory::InvalidExpression),
            ));
        }
        if Currency::parse(base).is_none() {
            return Some(finish(ctx, end, Err(IssueCategory::Unsupported)));
        }
        let number = &text[token.range.start..tokens[end - 1].range.end];
        return Some(finish(
            ctx,
            end,
            Quantity::parse(number, label.text)
                .map(Value::Quantity)
                .ok_or(IssueCategory::InvalidExpression),
        ));
    }
    if let Some((number, label)) = numeric::attached_quantity(source) {
        if !numeric::label(label) {
            return Some((Err(IssueCategory::Unsupported), index));
        }
        if Currency::parse(label).is_some()
            && source.starts_with(label)
            && split_suffix(source).is_some_and(|(_, suffix)| suffix.is_none())
            && number_fragment(number)
            && next.is_some_and(|next| {
                group_whitespace(text, token.range.end, next.range.start)
                    && split_suffix(next.text).is_some_and(|(body, _)| number_fragment(body))
            })
        {
            let end = amount_end(ctx, index + 1);
            let body = &text[token.range.start + label.len()..tokens[end].range.end];
            return Some(finish(
                ctx,
                end,
                prefixed_quantity(body, label)
                    .map(Value::Quantity)
                    .ok_or(IssueCategory::InvalidExpression),
            ));
        }
        let quantity = split_suffix(source)
            .and_then(|(_, suffix)| Quantity::parse_with_suffix(number, label, suffix));
        return Some(finish(
            ctx,
            index,
            quantity
                .map(Value::Quantity)
                .ok_or(IssueCategory::InvalidExpression),
        ));
    }
    let next = next?;
    if numeric::label(next.text) && source.chars().any(|c| c.is_ascii_digit()) {
        return Some(finish(
            ctx,
            index + 1,
            Quantity::parse(source, next.text)
                .map(Value::Quantity)
                .ok_or(IssueCategory::InvalidExpression),
        ));
    }
    if source.chars().any(|c| c.is_ascii_digit())
        && (lexicon::unit_marker(next.text)
            || (next.text.starts_with(char::is_alphabetic)
                && (next.text.contains(['/', '^', '²', '³'])
                    || ["Μg", "μG", "µG", "ug", "oz", "cl", "dl", "ms"].contains(&next.text))))
    {
        return Some((Err(IssueCategory::Unsupported), index + 1));
    }
    None
}

fn prefixed_quantity(number: &str, label: &str) -> Option<Quantity> {
    if matches!(label, "₺" | "$" | "€" | "£") {
        let (body, suffix) = split_suffix(number)?;
        Quantity::parse_with_suffix(body, label, suffix)
    } else {
        Quantity::parse(number, label)
    }
}

fn amount_end(ctx: &Context<'_>, index: usize) -> usize {
    let end = ctx.tokens[index].number_run_end;
    ctx.tokens
        .get(end + 1)
        .filter(|next| {
            group_whitespace(ctx.text, ctx.tokens[end].range.end, next.range.start)
                && split_suffix(next.text)
                    .is_some_and(|(body, suffix)| suffix.is_some() && number_fragment(body))
        })
        .map_or(end, |_| end + 1)
}

fn finish(ctx: &Context<'_>, end: usize, value: Result<Value, IssueCategory>) -> Attempt {
    if let Some(end) = quantity_math_end(ctx.text, ctx.tokens, end) {
        return (Err(IssueCategory::Unsupported), end);
    }
    if ctx.tokens.get(end + 1).is_some_and(|next| {
        quantity_tail(next.text)
            && whitespace_between(ctx.text, ctx.tokens[end].range.end, next.range.start)
    }) {
        return (Err(IssueCategory::InvalidExpression), end + 1);
    }
    (value, end)
}

fn contextual_range(ctx: &Context<'_>, index: usize) -> Option<Attempt> {
    if !number_fragment(ctx.tokens[index].text) {
        return None;
    }
    for end in index..=(index + 2).min(ctx.tokens.len().saturating_sub(1)) {
        let label = ctx.tokens.get(end + 1)?;
        if !ctx.tokens[index..=end]
            .iter()
            .all(|token| number_fragment(token.text) || matches!(token.text, "-" | "–"))
            || ctx.tokens[index..=end]
                .windows(2)
                .any(|pair| !whitespace_between(ctx.text, pair[0].range.end, pair[1].range.start))
        {
            continue;
        }
        let body = &ctx.text[ctx.tokens[index].range.start..ctx.tokens[end].range.end];
        if !body
            .char_indices()
            .any(|(i, ch)| i > 0 && matches!(ch, '-' | '–'))
        {
            continue;
        }
        if !whitespace_between(ctx.text, ctx.tokens[end].range.end, label.range.start)
            || !(lexicon::unit(label.text).is_some()
                || ["kişi", "adet", "gün", "yaş"]
                    .contains(&lexicon::lookup_key(label.text).as_str()))
        {
            continue;
        }
        return Some(finish(
            ctx,
            end + 1,
            NumericRange::parse(body, Some(label.text))
                .map(Value::Range)
                .ok_or(IssueCategory::InvalidExpression),
        ));
    }
    None
}

fn quantity_math_end(text: &str, tokens: &[Token<'_>], mut end: usize) -> Option<usize> {
    let initial = end;
    while let (Some(operator), Some(number)) = (tokens.get(end + 1), tokens.get(end + 2)) {
        if !super::super::scan::math_operator(operator.text)
            || !number.text.chars().any(|c| c.is_ascii_digit())
            || !whitespace_between(text, tokens[end].range.end, operator.range.start)
            || !whitespace_between(text, operator.range.end, number.range.start)
        {
            break;
        }
        end += 2;
        if tokens.get(end + 1).is_some_and(|tail| {
            numeric::label(tail.text)
                && whitespace_between(text, tokens[end].range.end, tail.range.start)
        }) {
            end += 1;
        }
    }
    (end > initial).then_some(end)
}
