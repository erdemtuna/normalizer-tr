use super::lexicon::{self, Currency, Lexeme};
use crate::{
    IssueCategory, SegmentKind,
    morphology::{Inflection, Spoken, case_inflection, integer_inflection, spoken_case},
    numerals::{self, Amount, Number},
};

#[derive(Clone, Debug)]
pub(crate) struct Numeric {
    number: Number,
    ordinal: bool,
    case: Option<Inflection>,
}

pub(crate) enum NumericPreference {
    Number(Numeric),
    SentenceNumber(Number),
}

pub(crate) struct NumericFailure {
    pub(crate) category: IssueCategory,
    pub(crate) preference: Option<NumericPreference>,
}

impl From<IssueCategory> for NumericFailure {
    fn from(category: IssueCategory) -> Self {
        Self {
            category,
            preference: None,
        }
    }
}

fn suffix_parts(text: &str) -> Option<(&str, Vec<&str>)> {
    let mut parts = text.split(['\'', '’']);
    let base = parts.next()?;
    let suffixes: Vec<_> = parts.collect();
    if suffixes.len() > 2 || suffixes.iter().any(|s| s.is_empty()) {
        return None;
    }
    Some((base, suffixes))
}

pub(crate) fn split_suffix(text: &str) -> Option<(&str, Option<&str>)> {
    let mut parts = text.split(['\'', '’']);
    let base = parts.next()?;
    let suffix = parts.next();
    if parts.next().is_some() || suffix.is_some_and(str::is_empty) {
        return None;
    }
    Some((base, suffix))
}

pub(crate) fn digits_hint(text: &str) -> Option<String> {
    let body = text.strip_prefix('+').unwrap_or(text);
    (body.bytes().any(|b| b.is_ascii_digit())
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '.' | '/' | '-' | '(' | ')')))
    .then(|| text.to_owned())
}

impl Numeric {
    pub(crate) fn automatic(text: &str) -> Result<Self, NumericFailure> {
        if text.chars().any(|c| c.is_numeric() && !c.is_ascii_digit()) {
            return Err(IssueCategory::Unsupported.into());
        }
        if let Some(value) = Self::parse(text, false) {
            return Ok(value);
        }
        let (base, suffix) = split_suffix(text).ok_or(IssueCategory::InvalidExpression)?;
        if base.ends_with('.') && suffix.is_none() {
            return Err(
                match Number::parse(&base[..base.len() - 1])
                    .filter(|number| number.fraction().is_empty() && !number.grouped())
                {
                    Some(number) => NumericFailure {
                        category: IssueCategory::Ambiguous,
                        preference: Some(NumericPreference::SentenceNumber(number)),
                    },
                    None => IssueCategory::InvalidExpression.into(),
                },
            );
        }
        let number = Number::parse(base).ok_or(IssueCategory::InvalidExpression)?;
        if number.grouped() {
            let case = suffix.map_or(Some(None), |suffix| {
                number
                    .fraction()
                    .is_empty()
                    .then(|| integer_inflection(&numerals::number(&number), suffix))
                    .flatten()
                    .map(Some)
            });
            return Err(NumericFailure {
                category: IssueCategory::Ambiguous,
                preference: case.map(|case| {
                    NumericPreference::Number(Self {
                        number,
                        ordinal: case == Some(Inflection::Ordinal),
                        case: case.filter(|family| *family != Inflection::Ordinal),
                    })
                }),
            });
        }
        let case = if let Some(suffix) = suffix {
            if !number.fraction().is_empty() {
                return Err(IssueCategory::Unsupported.into());
            }
            Some(
                integer_inflection(&numerals::number(&number), suffix)
                    .ok_or(IssueCategory::InvalidExpression)?,
            )
        } else {
            None
        };
        Ok(Self {
            number,
            ordinal: false,
            case,
        })
    }
    pub(crate) fn cardinal_hint(text: &str) -> Option<Self> {
        let (base, suffix) = split_suffix(text)?;
        let number = Number::parse_cardinal_hint(base)?;
        if !number.fraction().is_empty() {
            return None;
        }
        let family = suffix.map_or(Some(None), |s| {
            integer_inflection(&numerals::number(&number), s).map(Some)
        })?;
        Some(Self {
            number,
            ordinal: family == Some(Inflection::Ordinal),
            case: family.filter(|f| *f != Inflection::Ordinal),
        })
    }
    pub(crate) fn parse(text: &str, ordinal_hint: bool) -> Option<Self> {
        let (base, suffixes) = suffix_parts(text)?;
        let period = base.ends_with('.');
        if ordinal_hint && !period && suffixes.is_empty() {
            return None;
        }
        let body = if period {
            &base[..base.len() - 1]
        } else {
            base
        };
        let number = Number::parse(body)?;
        if !number.fraction().is_empty() || number.grouped() {
            return None;
        }
        let mut spoken = numerals::number(&number);
        let ordinal;
        let case;
        if period {
            if suffixes.is_empty() && !ordinal_hint {
                return None;
            }
            if suffixes.len() > 1 {
                return None;
            }
            spoken.inflect(Inflection::Ordinal);
            ordinal = true;
            case = suffixes
                .first()
                .map_or(Some(None), |s| spoken_case(&spoken, s).map(Some))?;
        } else if let Some(first) = suffixes.first() {
            let first_family = if let Some(family) = integer_inflection(&spoken, first) {
                family
            } else {
                if suffixes.len() != 1 {
                    return None;
                }
                let ordinal_suffix = spoken.source_suffix(Inflection::Ordinal);
                let remaining = first.strip_prefix(&ordinal_suffix)?;
                spoken.inflect(Inflection::Ordinal);
                return Some(Self {
                    number,
                    ordinal: true,
                    case: Some(spoken_case(&spoken, remaining)?),
                });
            };
            ordinal = first_family == Inflection::Ordinal;
            if ordinal {
                spoken.inflect(Inflection::Ordinal);
                case = suffixes
                    .get(1)
                    .map_or(Some(None), |s| spoken_case(&spoken, s).map(Some))?;
            } else {
                if ordinal_hint || suffixes.len() != 1 {
                    return None;
                }
                case = Some(first_family);
            }
        } else {
            ordinal = ordinal_hint;
            case = None;
        }
        Some(Self {
            number,
            ordinal,
            case,
        })
    }
    pub(crate) fn roman(text: &str) -> Option<Self> {
        let (base, suffixes) = suffix_parts(text)?;
        let ordinal = base.ends_with('.');
        let roman = base.strip_suffix('.').unwrap_or(base);
        let value = roman_value(roman)?;
        let mut numeric = value.to_string();
        if ordinal {
            numeric.push('.');
        }
        for suffix in &suffixes {
            numeric.push('\'');
            numeric.push_str(suffix);
        }
        let parsed = Self::parse(&numeric, ordinal)?;
        if !suffixes.is_empty() && !parsed.ordinal {
            return None;
        }
        Some(parsed)
    }
    pub(crate) fn render(&self) -> Spoken {
        let mut spoken = numerals::number(&self.number);
        if self.ordinal {
            spoken.inflect(Inflection::Ordinal);
        }
        if let Some(case) = self.case {
            spoken.inflect(case);
        }
        spoken
    }
    pub(crate) fn kind(&self) -> SegmentKind {
        if self.ordinal {
            SegmentKind::Ordinal
        } else if self.number.fraction().is_empty() {
            SegmentKind::Cardinal
        } else {
            SegmentKind::Decimal
        }
    }
}

fn roman_value(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 15 {
        return None;
    }
    let value = |b| match b {
        b'I' => Some(1_u16),
        b'V' => Some(5),
        b'X' => Some(10),
        b'L' => Some(50),
        b'C' => Some(100),
        b'D' => Some(500),
        b'M' => Some(1000),
        _ => None,
    };
    let mut total = 0_i32;
    let mut previous = 0;
    for b in text.bytes().rev() {
        let n = value(b)?;
        total += if n < previous {
            -(i32::from(n))
        } else {
            i32::from(n)
        };
        previous = n;
    }
    if !(1..=3999).contains(&total) {
        return None;
    }
    let mut rest = total;
    let mut canonical = String::new();
    for (n, s) in [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while rest >= n {
            canonical.push_str(s);
            rest -= n;
        }
    }
    (canonical == text).then_some(total as u16)
}

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

pub(crate) fn label(text: &str) -> bool {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    Currency::parse(base).is_some()
        || lexicon::unit(base).is_some()
        || lexicon::rate(base).is_some()
}

/// Surface ownership only; values and suffixes are validated by Quantity.
pub(crate) fn attached_quantity(text: &str) -> Option<(&str, &str)> {
    let base = text.split(['\'', '’']).next()?;
    if let Some(symbol) = base.chars().next().filter(|c| "₺$€£".contains(*c)) {
        return Some((&base[symbol.len_utf8()..], &base[..symbol.len_utf8()]));
    }
    if let Some(symbol) = base.chars().last().filter(|c| "₺$€£".contains(*c)) {
        let offset = base.len() - symbol.len_utf8();
        return Some((&base[..offset], &base[offset..]));
    }
    if !base.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '+' | '-')) {
        return None;
    }
    let offset = base.find(|c: char| c.is_alphabetic() || matches!(c, '°' | 'µ' | 'μ'))?;
    let tail = &base[offset..];
    (label(tail) || lexicon::unit_marker(tail) || unsupported_label(tail))
        .then_some((&base[..offset], tail))
}

pub(crate) fn quantity_piece(text: &str) -> bool {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    label(text)
        || attached_quantity(text)
            .is_some_and(|(number, label)| !number.is_empty() && base.ends_with(label))
}

pub(crate) fn currency_marker(text: &str) -> bool {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    Currency::parse(base).is_some()
        || (base.len() == 3
            && base.bytes().all(|byte| byte.is_ascii_uppercase())
            && lexicon::abbreviation(base).is_none()
            && !lexicon::unit_marker(base))
}

pub(crate) fn unsupported_label(text: &str) -> bool {
    matches!(
        text,
        "JPY"
            | "CHF"
            | "CAD"
            | "AUD"
            | "RUB"
            | "¥"
            | "Hz"
            | "kHz"
            | "MHz"
            | "GHz"
            | "°F"
            | "mph"
            | "MB"
            | "A"
            | "W"
    )
}

pub(crate) fn lexical_reading(
    text: &str,
) -> Option<Result<(Lexeme, Option<Inflection>), IssueCategory>> {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    let entry = lexicon::abbreviation(base)
        .or_else(|| Currency::parse(base).map(|currency| currency.lexeme(base)))?;
    let Some((_, suffixes)) = suffix_parts(text) else {
        return Some(Err(IssueCategory::Unsupported));
    };
    if suffixes.len() > 1 {
        return Some(Err(IssueCategory::Unsupported));
    }
    let case = match suffixes.first() {
        Some(suffix) => match case_inflection(entry.source, suffix) {
            Some(case) => Some(case),
            None => return Some(Err(IssueCategory::InvalidExpression)),
        },
        None => None,
    };
    Some(Ok((entry, case)))
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
