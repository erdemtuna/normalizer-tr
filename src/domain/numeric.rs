//! Exact number interpretations, ordinals and canonical Roman values.
use crate::{
    IssueCategory, SegmentKind,
    morphology::{Inflection, Spoken, integer_inflection, spoken_case},
    notation::{split_suffix, suffix_parts},
    numerals::{self, Number},
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
