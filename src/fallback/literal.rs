use unicode_segmentation::UnicodeSegmentation;

use super::{
    output::Output,
    spelling::{letter_name, symbol_name},
};
use crate::{
    NormalizeError,
    domain::lexicon::{self, Currency, Lexeme},
    numerals,
};

#[derive(Clone, Copy)]
pub(super) enum LetterReading {
    Spell,
    PreserveWords,
}

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

fn word_part(source: &str, reading: LetterReading) -> Part<'_> {
    if matches!(reading, LetterReading::Spell) {
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
    letters: LetterReading,
    quantities: bool,
    output: &mut Output<'_>,
) -> Result<bool, NormalizeError> {
    let mut cursor = 0;
    let mut used_code_point = false;
    let mut quantity_expected = false;
    while cursor < source.len() {
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
            let notation_length = remainder
                .find(|scalar: char| !scalar.is_ascii_digit() && !matches!(scalar, '.' | ','))
                .unwrap_or(remainder.len());
            if matches!(letters, LetterReading::PreserveWords)
                && let Some(number) = numerals::Number::parse(&remainder[..notation_length])
            {
                used_code_point |= emit(Part::Number(number), output)?;
                cursor += notation_length;
                quantity_expected = true;
                continue;
            }
            let length = remainder
                .find(|scalar: char| !scalar.is_ascii_digit())
                .unwrap_or(remainder.len());
            let digits = &remainder[..length];
            let part = match letters {
                LetterReading::Spell => Part::Digits(digits),
                LetterReading::PreserveWords => {
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
