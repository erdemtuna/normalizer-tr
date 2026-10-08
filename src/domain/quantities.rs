//! Validated money, measurements, percentages and contextual ranges.
use super::lexicon::{self, Currency, Lexeme};
use crate::{
    IssueCategory,
    morphology::{Inflection, Spoken, case_inflection, spoken_case},
    notation::{split_suffix, suffix_parts},
    numerals::{self, Amount, Number},
};

#[derive(Clone, Debug)]
enum QuantityValue {
    Number(Number),
    Money(Amount, Currency),
}

#[derive(Clone, Debug)]
pub(crate) struct Quantity {
    value: QuantityValue,
    lexeme: Lexeme,
    prefix: Option<Lexeme>,
    case: Option<Inflection>,
}

impl Quantity {
    pub(crate) fn parse(number: &str, label: &str) -> Option<Self> {
        let (base, suffix) = split_suffix(label)?;
        Self::parse_with_suffix(number, base, suffix)
    }
    pub(crate) fn parse_with_suffix(
        number: &str,
        base: &str,
        suffix: Option<&str>,
    ) -> Option<Self> {
        let (value, lexeme, prefix) = if let Some(currency) = Currency::parse(base) {
            (
                QuantityValue::Money(Amount::parse(number)?, currency),
                currency.lexeme(base),
                None,
            )
        } else if let Some((prefix, lexeme)) = lexicon::rate(base) {
            if suffix.is_some() {
                return None;
            }
            (
                QuantityValue::Number(Number::parse(number)?),
                lexeme,
                Some(prefix),
            )
        } else {
            (
                QuantityValue::Number(Number::parse(number)?),
                lexicon::unit(base)?,
                None,
            )
        };
        let case = suffix.map_or(Some(None), |s| case_inflection(lexeme.source, s).map(Some))?;
        Some(Self {
            value,
            lexeme,
            prefix,
            case,
        })
    }
    pub(crate) fn render(&self) -> Spoken {
        let mut spoken = match &self.value {
            QuantityValue::Number(number) => {
                let text = format!(
                    "{} {}",
                    numerals::number(number).into_text(),
                    self.lexeme.output
                );
                Spoken::lexical(&text, self.lexeme.target)
            }
            QuantityValue::Money(amount, currency) => numerals::amount(
                amount,
                self.lexeme.output,
                self.lexeme.target,
                currency.minor().target,
            ),
        };
        if let Some(prefix) = self.prefix {
            let mut denominator = Spoken::lexical(prefix.output, prefix.target);
            denominator.inflect(Inflection::Locative);
            spoken.prefix(&format!("{} ", denominator.into_text()));
        }
        if let Some(case) = self.case {
            spoken.inflect(case);
        }
        spoken
    }
    pub(crate) fn is_money(&self) -> bool {
        matches!(self.value, QuantityValue::Money(..))
    }
}

pub(crate) fn percent(text: &str) -> Option<Result<(Number, Option<Inflection>), IssueCategory>> {
    let (base, suffixes) = suffix_parts(text)?;
    let body = base.strip_prefix('%').or_else(|| base.strip_suffix('%'))?;
    Some(percent_value(body, &suffixes))
}

pub(crate) fn percent_body(text: &str) -> Result<(Number, Option<Inflection>), IssueCategory> {
    let (body, suffixes) = suffix_parts(text).ok_or(IssueCategory::Unsupported)?;
    percent_value(body, &suffixes)
}

fn percent_value(
    body: &str,
    suffixes: &[&str],
) -> Result<(Number, Option<Inflection>), IssueCategory> {
    let number = match Number::parse(body) {
        Some(n) => n,
        None => return Err(IssueCategory::InvalidExpression),
    };
    if suffixes.len() > 1 {
        return Err(IssueCategory::Unsupported);
    }
    let spoken = numerals::number(&number);
    let inflection = match suffixes.first() {
        Some(s) if *s == spoken.source_suffix(Inflection::Derivation) => {
            Some(Inflection::Derivation)
        }
        Some(s) => match spoken_case(&spoken, s) {
            Some(case) => Some(case),
            None => return Err(IssueCategory::InvalidExpression),
        },
        None => None,
    };
    // Keep the validated number and grammatical family, not parsed emitted text.
    Ok((number, inflection))
}

#[derive(Clone, Debug)]
pub(crate) struct NumericRange {
    start: Number,
    end: Number,
    context: Option<Lexeme>,
    noun: Option<String>,
}

impl NumericRange {
    pub(crate) fn parse(text: &str, context: Option<&str>) -> Option<Self> {
        let mut found = None;
        for (offset, ch) in text.char_indices() {
            if !matches!(ch, '-' | '–') || offset == 0 {
                continue;
            }
            if let (Some(start), Some(end)) = (
                Number::parse(text[..offset].trim()),
                Number::parse(text[offset + ch.len_utf8()..].trim()),
            ) {
                if found.is_some() {
                    return None;
                }
                found = Some((start, end));
            }
        }
        let (start, end) = found?;
        let (unit, noun) = match context {
            Some(value)
                if ["kişi", "adet", "gün", "yaş"]
                    .contains(&lexicon::lookup_key(value).as_str()) =>
            {
                (None, Some(value.to_owned()))
            }
            Some(value) => (Some(lexicon::unit(value)?), None),
            None => (None, None),
        };
        Some(Self {
            start,
            end,
            context: unit,
            noun,
        })
    }
    pub(crate) fn render(&self) -> String {
        let mut result = format!(
            "{} ila {}",
            numerals::number(&self.start).into_text(),
            numerals::number(&self.end).into_text()
        );
        if let Some(unit) = self.context {
            result.push(' ');
            result.push_str(unit.output);
        }
        if let Some(noun) = &self.noun {
            result.push(' ');
            result.push_str(noun);
        }
        result
    }
}
