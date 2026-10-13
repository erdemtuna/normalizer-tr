use unicode_segmentation::UnicodeSegmentation;

use super::output::Output;
use crate::notation::{letter_name, symbol_name};
use crate::{
    NormalizeError,
    domain::lexicon::{self, Currency, Lexeme},
    numerals,
};

use crate::interpretation::LiteralReading;

enum Part<'a> {
    Number(numerals::Number),
    Digits(&'a str),
    Letters(&'a str),
    Word(&'a str),
    Label(Lexeme),
    Symbol(&'a str),
}

fn emit(part: Part<'_>, output: &mut Output<'_>) -> Result<bool, NormalizeError> {
    match part {
        Part::Number(number) => output.word(&numerals::number(&number).into_text())?,
        Part::Digits(digits) => {
            for digit in digits.chars() {
                output.word(numerals::digit_name(digit).ok_or(NormalizeError::Internal)?)?;
            }
        }
        Part::Word(word) => output.word(word)?,
        Part::Label(label) => output.word(label.output)?,
        Part::Letters(letters) => {
            let mut used_code_point = false;
            for letter in letters.chars() {
                if let Some(name) = letter_name(letter).or_else(|| symbol_name(letter)) {
                    output.word(name)?;
                } else {
                    code_point(letter, output)?;
                    used_code_point = true;
                }
            }
            return Ok(used_code_point);
        }
        Part::Symbol(grapheme) => {
            let mut scalars = grapheme.chars();
            let first = scalars.next().ok_or(NormalizeError::Internal)?;
            if scalars.next().is_none()
                && let Some(name) = symbol_name(first)
            {
                output.word(name)?;
                return Ok(false);
            }
            // A multi-scalar cluster is owned as a whole, with every scalar retained.
            for scalar in grapheme.chars() {
                code_point(scalar, output)?;
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn code_point(scalar: char, output: &mut Output<'_>) -> Result<(), NormalizeError> {
    output.word("unikod u artı")?;
    for hexadecimal in format!("{:04X}", u32::from(scalar)).chars() {
        let name = numerals::digit_name(hexadecimal)
            .or_else(|| letter_name(hexadecimal))
            .ok_or(NormalizeError::Internal)?;
        output.word(name)?;
    }
    Ok(())
}

fn word_part(source: &str, reading: LiteralReading) -> Part<'_> {
    if matches!(reading, LiteralReading::Spell) {
        return Part::Letters(source);
    }
    let label = lexicon::abbreviation(source)
        .or_else(|| Currency::parse(source).map(|currency| currency.lexeme(source)));
    if let Some(label) = label {
        Part::Label(label)
    } else {
        Part::Word(source)
    }
}

/// Ordered borrowed runs. Numeric interpretation never crosses a delimiter.
pub(super) fn render(
    source: &str,
    letters: LiteralReading,
    quantities: bool,
    output: &mut Output<'_>,
) -> Result<bool, NormalizeError> {
    let mut cursor = 0;
    let mut used_code_point = false;
    let mut quantity_expected = false;
    let mut notation_end = 0;
    while cursor < source.len() {
        output.check()?;
        let remainder = &source[cursor..];
        let first = remainder.chars().next().ok_or(NormalizeError::Internal)?;
        if first.is_whitespace() {
            let end = remainder
                .find(|scalar: char| !scalar.is_whitespace())
                .unwrap_or(remainder.len());
            output.append(&remainder[..end])?;
            cursor += end;
            continue;
        }
        if quantities
            && quantity_expected
            && let Some((symbol, label)) = lexicon::unit_prefix(remainder)
        {
            used_code_point |= emit(Part::Label(label), output)?;
            cursor += symbol.len();
            quantity_expected = false;
            continue;
        }
        let (part, length) = if first.is_ascii_digit() {
            if matches!(letters, LiteralReading::PreserveWords) {
                if cursor >= notation_end {
                    notation_end = cursor
                        + remainder
                            .bytes()
                            .position(|byte| !byte.is_ascii_digit() && !matches!(byte, b'.' | b','))
                            .unwrap_or(remainder.len());
                }
                if let Some(number) = numerals::Number::parse(&source[cursor..notation_end]) {
                    used_code_point |= emit(Part::Number(number), output)?;
                    cursor = notation_end;
                    quantity_expected = true;
                    continue;
                }
            }
            let length = remainder
                .find(|scalar: char| !scalar.is_ascii_digit())
                .unwrap_or(remainder.len());
            let digits = &remainder[..length];
            let part = match letters {
                LiteralReading::Spell => Part::Digits(digits),
                LiteralReading::PreserveWords => {
                    numerals::Number::parse(digits).map_or(Part::Digits(digits), Part::Number)
                }
            };
            (part, length)
        } else if first.is_alphabetic() && symbol_name(first).is_none() {
            let length = remainder
                .find(|scalar: char| !scalar.is_alphabetic())
                .unwrap_or(remainder.len());
            let word = &remainder[..length];
            let part = word_part(word, letters);
            (part, length)
        } else {
            let grapheme = remainder
                .graphemes(true)
                .next()
                .ok_or(NormalizeError::Internal)?;
            (Part::Symbol(grapheme), grapheme.len())
        };
        quantity_expected = matches!(&part, Part::Number(_) | Part::Digits(_));
        used_code_point |= emit(part, output)?;
        cursor += length;
    }
    Ok(used_code_point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LimitKind, WorkControl};
    use std::time::Instant;

    fn standard_code_point(scalar: char, output: &mut Output<'_>) -> Result<(), NormalizeError> {
        output.word("unikod u artı")?;
        for hexadecimal in format!("{:04X}", u32::from(scalar)).chars() {
            let name = numerals::digit_name(hexadecimal)
                .or_else(|| letter_name(hexadecimal))
                .ok_or(NormalizeError::Internal)?;
            output.word(name)?;
        }
        Ok(())
    }

    #[test]
    fn code_point_width_padding_order_and_scalar_extremes_are_exact() {
        for (value, digits) in [
            (0, "sıfır sıfır sıfır sıfır"),
            (1, "sıfır sıfır sıfır bir"),
            (9, "sıfır sıfır sıfır dokuz"),
            (0xA, "sıfır sıfır sıfır a"),
            (0xB, "sıfır sıfır sıfır be"),
            (0xC, "sıfır sıfır sıfır ce"),
            (0xD, "sıfır sıfır sıfır de"),
            (0xE, "sıfır sıfır sıfır e"),
            (0xF, "sıfır sıfır sıfır fe"),
            (0xFF, "sıfır sıfır fe fe"),
            (0xFFF, "sıfır fe fe fe"),
            (0x1000, "bir sıfır sıfır sıfır"),
            (0xD7FF, "de yedi fe fe"),
            (0xE000, "e sıfır sıfır sıfır"),
            (0xFFFF, "fe fe fe fe"),
            (0x10000, "bir sıfır sıfır sıfır sıfır"),
            (0x1FAE0, "bir fe a e sıfır"),
            (0xABCDE, "a be ce de e"),
            (0xFFFFF, "fe fe fe fe fe"),
            (0x100000, "bir sıfır sıfır sıfır sıfır sıfır"),
            (0x10FFFF, "bir sıfır fe fe fe fe"),
        ] {
            let scalar = char::from_u32(value).unwrap();
            let expected = format!("unikod u artı {digits}");
            let control = WorkControl::default();
            let mut output = Output::new(expected.len(), &control);
            assert_eq!(code_point(scalar, &mut output), Ok(()), "U+{value:X}");
            assert_eq!(output.finish(), expected, "U+{value:X}");
        }
        for surrogate in 0xD800..=0xDFFF {
            assert!(char::from_u32(surrogate).is_none());
        }
    }

    #[test]
    fn code_point_allowance_cutoffs_match_standard_emission_and_partial_text() {
        for scalar in ['\u{301}', '\u{1FAE0}', '\u{10FFFF}'] {
            for prefix in ["", "ö", "ö "] {
                let control = WorkControl::default();
                let mut reference = Output::new(usize::MAX, &control);
                reference.append(prefix).unwrap();
                standard_code_point(scalar, &mut reference).unwrap();
                let expected = reference.finish();
                for maximum in prefix.len()..=expected.len() {
                    let mut actual = Output::new(maximum, &control);
                    let mut reference = Output::new(maximum, &control);
                    actual.append(prefix).unwrap();
                    reference.append(prefix).unwrap();
                    let result = code_point(scalar, &mut actual);
                    assert_eq!(result, standard_code_point(scalar, &mut reference));
                    assert_eq!(
                        result,
                        if maximum == expected.len() {
                            Ok(())
                        } else {
                            Err(NormalizeError::LimitExceeded(LimitKind::Result))
                        }
                    );
                    assert_eq!(actual.finish(), reference.finish(), "{scalar:?} {maximum}");
                }
            }
        }
    }

    #[test]
    fn code_point_cancellation_precedes_an_insufficient_allowance() {
        for prefix in ["", "ö"] {
            let control = WorkControl::default();
            let mut output = Output::new(prefix.len(), &control);
            output.append(prefix).unwrap();
            control.cancel();
            assert_eq!(
                code_point('\u{1FAE0}', &mut output),
                Err(NormalizeError::Cancelled)
            );
            assert_eq!(output.finish(), prefix);
        }
        let expired = WorkControl::new(Some(Instant::now()));
        let mut output = Output::new(0, &expired);
        assert_eq!(
            code_point('\u{10FFFF}', &mut output),
            Err(NormalizeError::Cancelled)
        );
        assert_eq!(output.finish(), "");
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(256))]
        #[test]
        fn arbitrary_scalar_matches_the_standard_format_oracle(
            scalar in proptest::prelude::any::<char>(),
            maximum in 0_usize..100,
            prefix in proptest::sample::select(vec!["", "ö", "ö "]),
        ) {
            let control = WorkControl::default();
            let maximum = maximum + prefix.len();
            let mut actual = Output::new(maximum, &control);
            let mut reference = Output::new(maximum, &control);
            actual.append(prefix).unwrap();
            reference.append(prefix).unwrap();
            proptest::prop_assert_eq!(
                code_point(scalar, &mut actual),
                standard_code_point(scalar, &mut reference)
            );
            proptest::prop_assert_eq!(actual.finish(), reference.finish());
        }
    }
}
