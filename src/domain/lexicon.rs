mod abbreviations;
mod pronunciations;
mod static_index;

use crate::morphology::{Harmony, Word, WordEnd};
use crate::{
    IssueCategory,
    morphology::{Inflection, case_inflection},
    notation::split_suffix,
};

use Harmony::{BackFlat, BackRound, FrontFlat, FrontRound};
use WordEnd::{Possessive, SoftensP, Voiced, Voiceless, Vowel};

pub(crate) const MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];

pub(crate) fn month_name(month: u8) -> Option<&'static str> {
    month
        .checked_sub(1)
        .and_then(|index| MONTHS.get(usize::from(index)))
        .copied()
}

/// Turkish casing is confined to explicit contextual lookup keys, never source rewriting.
pub(crate) fn lookup_key(text: &str) -> String {
    text.chars()
        .flat_map(|ch| match ch {
            'I' => 'ı'.to_lowercase(),
            'İ' => 'i'.to_lowercase(),
            _ => ch.to_lowercase(),
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Lexeme {
    pub(crate) output: &'static str,
    pub(crate) source: Word,
    pub(crate) target: Word,
}

impl Lexeme {
    const fn same(output: &'static str, harmony: Harmony, end: WordEnd) -> Self {
        let word = Word::new(output, harmony, end);
        Self {
            output,
            source: word,
            target: word,
        }
    }
    const fn distinct(output: &'static str, source: Word, target: Word) -> Self {
        Self {
            output,
            source,
            target,
        }
    }
}

const UNITS: &[(&str, Lexeme)] = &[
    ("kg", Lexeme::same("kilogram", BackFlat, Voiced)),
    ("g", Lexeme::same("gram", BackFlat, Voiced)),
    ("gr", Lexeme::same("gram", BackFlat, Voiced)),
    ("mg", Lexeme::same("miligram", BackFlat, Voiced)),
    ("µg", Lexeme::same("mikrogram", BackFlat, Voiced)),
    ("μg", Lexeme::same("mikrogram", BackFlat, Voiced)),
    ("km", Lexeme::same("kilometre", FrontFlat, Vowel)),
    ("m", Lexeme::same("metre", FrontFlat, Vowel)),
    ("cm", Lexeme::same("santimetre", FrontFlat, Vowel)),
    ("mm", Lexeme::same("milimetre", FrontFlat, Vowel)),
    ("L", Lexeme::same("litre", FrontFlat, Vowel)),
    ("lt", Lexeme::same("litre", FrontFlat, Vowel)),
    ("mL", Lexeme::same("mililitre", FrontFlat, Vowel)),
    ("ml", Lexeme::same("mililitre", FrontFlat, Vowel)),
    ("dk", Lexeme::same("dakika", BackFlat, Vowel)),
    ("sn", Lexeme::same("saniye", FrontFlat, Vowel)),
    ("sa", Lexeme::same("saat", FrontFlat, Voiceless)),
    ("m²", Lexeme::same("metrekare", FrontFlat, Vowel)),
    ("cm²", Lexeme::same("santimetrekare", FrontFlat, Vowel)),
    ("km²", Lexeme::same("kilometrekare", FrontFlat, Vowel)),
    ("m³", Lexeme::same("metreküp", FrontRound, SoftensP)),
    (
        "°C",
        Lexeme::distinct(
            "derece Santigrat",
            Word::new("Santigrat", BackFlat, Voiceless),
            Word::new("Santigrat", BackFlat, Voiceless),
        ),
    ),
    ("V", Lexeme::same("volt", BackRound, Voiceless)),
    ("kW", Lexeme::same("kilovat", BackFlat, Voiceless)),
    (
        "kWh",
        Lexeme::distinct(
            "kilovat saat",
            Word::new("saat", FrontFlat, Voiceless),
            Word::new("saat", FrontFlat, Voiceless),
        ),
    ),
    ("GB", Lexeme::same("gigabayt", BackFlat, Voiceless)),
];

pub(crate) fn unit(symbol: &str) -> Option<Lexeme> {
    UNITS
        .iter()
        .find(|(key, _)| *key == symbol)
        .map(|(_, value)| *value)
}

/// Detection of an unsupported spelling is not acceptance or source case rewriting.
pub(crate) fn unit_marker(symbol: &str) -> bool {
    UNITS
        .iter()
        .any(|(key, _)| key.eq_ignore_ascii_case(symbol))
}

pub(crate) fn unit_prefix(text: &str) -> Option<(&str, Lexeme)> {
    UNITS.iter().find_map(|(symbol, entry)| {
        text.strip_prefix(symbol)
            .filter(|rest| !rest.starts_with(|ch: char| ch.is_alphanumeric() || ch == '_'))
            .map(|_| (*symbol, *entry))
    })
}

pub(crate) fn rate(symbol: &str) -> Option<(Lexeme, Lexeme)> {
    match symbol {
        "km/sa" | "km/h" => Some((unit("sa")?, unit("km")?)),
        "m/s" => Some((unit("sn")?, unit("m")?)),
        _ => None,
    }
}

pub(crate) fn abbreviation(symbol: &str) -> Option<Lexeme> {
    abbreviations::lookup(symbol).map(abbreviations::Definition::default)
}

pub(crate) fn plain_abbreviation(text: &str) -> bool {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    abbreviation(base).is_some() && base.chars().all(char::is_uppercase)
}

pub(crate) fn pronunciation_candidates(first: &str) -> &'static [(&'static str, &'static Lexeme)] {
    pronunciations::candidates(first)
}

pub(crate) fn pronunciation_word(text: &str) -> bool {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    pronunciations::lookup(base).is_some()
}

pub(crate) fn pronunciation_boundary(text: &str) -> bool {
    pronunciations::boundary_word(text)
}

pub(crate) fn pronunciation_reading(
    entry: Lexeme,
    text: &str,
) -> Result<(Lexeme, Option<Inflection>), IssueCategory> {
    let (_, suffix) = split_suffix(text).ok_or(IssueCategory::Unsupported)?;
    let case = suffix
        .map(|suffix| case_inflection(entry.source, suffix).ok_or(IssueCategory::InvalidExpression))
        .transpose()?;
    Ok((entry, case))
}

pub(crate) fn catalog_form_valid(text: &str) -> bool {
    if let Some(reading) = lexical_reading(text) {
        return reading.is_ok();
    }
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    pronunciations::lookup(base).is_some_and(|entry| pronunciation_reading(entry, text).is_ok())
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Currency {
    Try,
    Usd,
    Eur,
    Gbp,
}

impl Currency {
    pub(crate) fn parse(label: &str) -> Option<Self> {
        match label {
            "TL" | "TRY" | "₺" => Some(Self::Try),
            "USD" | "$" => Some(Self::Usd),
            "EUR" | "€" => Some(Self::Eur),
            "GBP" | "£" => Some(Self::Gbp),
            _ => None,
        }
    }
    pub(crate) fn lexeme(self, label: &str) -> Lexeme {
        match self {
            Self::Try => Lexeme::distinct(
                "Türk lirası",
                if label == "TRY" {
                    Word::new("ye", FrontFlat, Vowel)
                } else if label == "₺" {
                    Word::new("lira", BackFlat, Vowel)
                } else {
                    Word::new("le", FrontFlat, Vowel)
                },
                Word::new("lirası", BackFlat, Possessive),
            ),
            Self::Usd => Lexeme::distinct(
                "dolar",
                if label == "USD" {
                    Word::new("de", FrontFlat, Vowel)
                } else {
                    Word::new("dolar", BackFlat, Voiced)
                },
                Word::new("dolar", BackFlat, Voiced),
            ),
            Self::Eur => Lexeme::distinct(
                "avro",
                if label == "EUR" {
                    Word::new("re", FrontFlat, Vowel)
                } else {
                    Word::new("avro", BackRound, Vowel)
                },
                Word::new("avro", BackRound, Vowel),
            ),
            Self::Gbp => Lexeme::distinct(
                "sterlin",
                if label == "GBP" {
                    Word::new("pe", FrontFlat, Vowel)
                } else {
                    Word::new("sterlin", FrontFlat, Voiced)
                },
                Word::new("sterlin", FrontFlat, Voiced),
            ),
        }
    }
    pub(crate) fn minor(self) -> Lexeme {
        match self {
            Self::Try => Lexeme::same("kuruş", BackRound, Voiceless),
            Self::Usd | Self::Eur => Lexeme::same("sent", FrontFlat, Voiceless),
            Self::Gbp => Lexeme::same("peni", FrontFlat, Vowel),
        }
    }
}

pub(crate) fn lexical_reading(
    text: &str,
) -> Option<Result<(Lexeme, Option<Inflection>), IssueCategory>> {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    let definition = abbreviations::lookup(base);
    let entry = definition
        .map(abbreviations::Definition::default)
        .or_else(|| Currency::parse(base).map(|currency| currency.lexeme(base)))?;
    let Some((_, suffix)) = split_suffix(text) else {
        return Some(Err(IssueCategory::Unsupported));
    };
    let Some(suffix) = suffix else {
        return Some(Ok((entry, None)));
    };
    let selected = if let Some(definition) = definition {
        definition.inflected(suffix)
    } else {
        case_inflection(entry.source, suffix).map(|case| (entry, case))
    };
    Some(
        selected
            .map(|(entry, case)| (entry, Some(case)))
            .ok_or(IssueCategory::InvalidExpression),
    )
}
