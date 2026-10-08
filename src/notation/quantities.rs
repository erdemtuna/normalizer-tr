//! Quantity-shaped source ownership, distinct from value validation.
use crate::domain::lexicon::{self, Currency};

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
