//! Shared written-form syntax and noncertifying source representations.
mod quantities;
mod suffix;
mod symbols;
mod temporal;

pub(crate) use quantities::{
    attached_quantity, currency_marker, label, quantity_piece, unsupported_label,
};
pub(crate) use suffix::{split_suffix, suffix_parts};
pub(crate) use symbols::{emoticon_length, letter_name, needs_reading, symbol_name};
pub(crate) use temporal::{DateSurface, TimeSurface};

pub(crate) fn digits_hint(text: &str) -> Option<String> {
    let body = text.strip_prefix('+').unwrap_or(text);
    (body.bytes().any(|b| b.is_ascii_digit())
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '.' | '/' | '-' | '(' | ')')))
    .then(|| text.to_owned())
}
