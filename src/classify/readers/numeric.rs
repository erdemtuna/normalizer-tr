use super::super::scan::whitespace_between;
use super::{Attempt, Context};
use crate::{
    FallbackClass, IssueCategory,
    domain::{
        lexicon,
        numeric::{self, Numeric},
    },
    model::Value,
};
pub(super) fn read(ctx: &Context<'_>, index: usize) -> Option<(Attempt, FallbackClass)> {
    let text = ctx.text;
    let tokens = ctx.tokens;
    let token = tokens[index];
    let source = token.text;
    if source == "%"
        && let Some(next) = tokens.get(index + 1).filter(|next| {
            whitespace_between(text, token.range.end, next.range.start)
                && next.text.chars().any(|ch| ch.is_ascii_digit())
        })
    {
        return Some((
            (
                numeric::percent_body(next.text).map(|(number, case)| Value::Percent(number, case)),
                index + 1,
            ),
            FallbackClass::Percent,
        ));
    }
    if let Some(percent) = numeric::percent(source) {
        return Some((
            (
                percent.map(|(number, case)| Value::Percent(number, case)),
                index,
            ),
            FallbackClass::Percent,
        ));
    }
    {
        let base = source.split(['\'', '’']).next().unwrap_or(source);
        if (base.ends_with('.') || source.contains(['\'', '’']))
            && let Some(number) = Numeric::parse(source, false)
        {
            return Some(((Ok(Value::Numeric(number)), index), FallbackClass::Number));
        }
    }
    let roman_base = source
        .split(['\'', '’'])
        .next()
        .unwrap_or(source)
        .trim_end_matches('.');
    if !roman_base.is_empty() && roman_base.bytes().all(|b| b"IVXLCDM".contains(&b)) {
        let contextual = source.ends_with('.')
            && tokens.get(index + 1).is_some_and(|next| {
                whitespace_between(text, token.range.end, next.range.start)
                    && (lexicon::lookup_key(next.text) == "yüzyıl"
                        || (lexicon::lookup_key(next.text) == "dünya"
                            && tokens.get(index + 2).is_some_and(|last| {
                                lexicon::lookup_key(last.text) == "savaşı"
                                    && whitespace_between(text, next.range.end, last.range.start)
                            })))
            });
        return Some((
            (
                if contextual {
                    Numeric::roman(source)
                        .map(Value::Roman)
                        .ok_or(IssueCategory::InvalidExpression)
                } else {
                    Err(IssueCategory::Ambiguous)
                },
                index,
            ),
            FallbackClass::Roman,
        ));
    }

    None
}
