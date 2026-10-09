//! Incremental source-piece signals; semantic quantity validation stays in domain.
use crate::notation;

#[derive(Default)]
pub(super) struct Piece {
    prefix_currency: Option<char>,
    last: Option<char>,
    label: bool,
    broken_label: bool,
    letters: bool,
    digits: bool,
    underscore: bool,
    quoted_quantity: Option<bool>,
}

impl Piece {
    pub(super) fn quantity(&self, source: &str) -> bool {
        if let Some(quantity) = self.quoted_quantity {
            return quantity;
        }
        if let Some(currency) = self.prefix_currency {
            return self.last == Some(currency);
        }
        if self.last.is_some_and(currency_symbol) {
            return true;
        }
        self.label && !self.broken_label && notation::quantity_piece(source)
    }

    pub(super) fn identifier(&self, source: &str) -> bool {
        source.starts_with("www.") || (self.digits && (self.letters || self.underscore))
    }

    pub(super) fn advance(&mut self, preceding: &str, ch: char) {
        if self.quoted_quantity.is_some() {
            return;
        }
        if matches!(ch, '\'' | '’') {
            self.quoted_quantity = Some(self.quantity(preceding));
            return;
        }
        if self.last.is_none() {
            self.prefix_currency = currency_symbol(ch).then_some(ch);
        }
        self.letters |= ch.is_alphabetic() && !ch.is_numeric();
        self.digits |= ch.is_numeric();
        self.underscore |= ch == '_';
        if self.label && !(ch.is_alphabetic() || matches!(ch, '/' | '°' | '²' | '³')) {
            self.broken_label = true;
        }
        self.label |= ch.is_alphabetic() || ch == '°';
        self.last = Some(ch);
    }
}

fn currency_symbol(ch: char) -> bool {
    matches!(ch, '₺' | '$' | '€' | '£')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) {
        let mut piece = Piece::default();
        for (offset, ch) in source.char_indices() {
            let prefix = &source[..offset];
            assert_eq!(
                piece.quantity(prefix),
                notation::quantity_piece(prefix),
                "{prefix:?}"
            );
            assert_eq!(
                piece.identifier(prefix),
                super::super::identifier(prefix),
                "{prefix:?}"
            );
            piece.advance(prefix, ch);
        }
        assert_eq!(
            piece.quantity(source),
            notation::quantity_piece(source),
            "{source:?}"
        );
        assert_eq!(
            piece.identifier(source),
            super::super::identifier(source),
            "{source:?}"
        );
    }

    #[test]
    fn incremental_signals_match_existing_surface_ownership() {
        for source in [
            "",
            "1,1,1,",
            "25,30kg",
            "25kg",
            "25kg'12",
            "25kg'xx,1",
            "1 234,50TL",
            "TL'ye",
            "kg",
            "5°C",
            "5µg",
            "5μg",
            "10m²",
            "2cm²",
            "2m³",
            "25km/sa",
            "$1",
            "$1$",
            "£12",
            "€x$",
            "foo$",
            "$1,x$",
            "AB12(25kg)",
            "5kg,not,1",
            "foo_bar",
            "www.example.com",
            "NATO'dan",
            "25KG",
            "5°",
            "5°C'xx,1",
            "5kg'12,1",
            "5°x,1",
            "5m^2",
        ] {
            check(source);
        }
    }

    #[test]
    fn numeric_prefixes_never_require_quantity_reinspection() {
        let source = "1,".repeat(16384);
        let mut piece = Piece::default();
        for (offset, ch) in source.char_indices() {
            assert!(!piece.label && !piece.broken_label && piece.quoted_quantity.is_none());
            assert!(!piece.quantity(&source[..offset]));
            piece.advance(&source[..offset], ch);
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(128))]
        #[test]
        fn piece_signals_match_surface_helpers_for_generated_prefixes(
            chars in proptest::collection::vec(
                proptest::sample::select(vec![
                    '1', '2', '0', 'A', 'g', 'm', 'k', 'w', 'µ', 'μ', '²', '³',
                    '°', '$', '€', '£', '₺', '\'', '’', '/', '_', '.', ',', '+', '-',
                ]),
                0..64
            )
        ) {
            check(&chars.into_iter().collect::<String>());
        }
    }
}
