mod literal;
mod output;
mod spelling;
mod surface;

use crate::{
    FallbackClass, FallbackReason, FallbackStrategy, IssueCategory, NormalizeError, WorkControl,
    domain::numeric::{NumericFailure, NumericPreference},
    model::{TemporalFailure, TemporalPreference, Value},
    numerals, verbalize,
};
use literal::LetterReading;
use output::Output;
use surface::{DateSurface, TimeSurface};

enum Prepared {
    Number(NumericPreference),
    Temporal(TemporalPreference),
    Date(DateSurface),
    Time(TimeSurface),
    Literal(LetterReading),
}

/// Keeps the failed family and readable source alternative together.
pub(crate) struct Request {
    category: Option<IssueCategory>,
    class: FallbackClass,
    reason: FallbackReason,
    prepared: Prepared,
}

impl Request {
    pub(crate) fn category(&self) -> Option<IssueCategory> {
        self.category
    }
    pub(crate) fn class(&self) -> FallbackClass {
        self.class
    }
    pub(crate) fn reason(&self) -> FallbackReason {
        self.reason
    }

    pub(crate) fn unresolved(source: &str, class: FallbackClass, category: IssueCategory) -> Self {
        let prepared = match class {
            FallbackClass::Date => DateSurface::parse(source).map(Prepared::Date),
            FallbackClass::Time => TimeSurface::parse(source).map(Prepared::Time),
            _ => None,
        };
        let spell = matches!(
            class,
            FallbackClass::Identifier
                | FallbackClass::Abbreviation
                | FallbackClass::Roman
                | FallbackClass::Electronic
        );
        Self::primary(
            source,
            class,
            category,
            prepared.unwrap_or(Prepared::Literal(if spell {
                LetterReading::Spell
            } else {
                LetterReading::PreserveWords
            })),
        )
    }

    fn primary(
        source: &str,
        class: FallbackClass,
        category: IssueCategory,
        prepared: Prepared,
    ) -> Self {
        let reason = if source.len() > 1
            && source.starts_with('0')
            && source.bytes().all(|byte| byte.is_ascii_digit())
        {
            FallbackReason::LeadingZeroes
        } else {
            match category {
                IssueCategory::Ambiguous => FallbackReason::MissingIntent,
                IssueCategory::InvalidExpression => FallbackReason::InvalidForm,
                IssueCategory::ProtectedIdentifier => FallbackReason::ProtectedIdentifier,
                IssueCategory::Unsupported => FallbackReason::UnsupportedForm,
                IssueCategory::UnknownAbbreviation => FallbackReason::UnapprovedAbbreviation,
            }
        };
        Self {
            category: Some(category),
            class,
            reason,
            prepared,
        }
    }

    pub(crate) fn symbols() -> Self {
        Self {
            category: None,
            class: FallbackClass::Symbol,
            reason: FallbackReason::UnhandledSymbol,
            prepared: Prepared::Literal(LetterReading::Spell),
        }
    }

    pub(crate) fn numeric(source: &str, failure: NumericFailure) -> Self {
        Self::primary(
            source,
            FallbackClass::Number,
            failure.category,
            failure.preference.map_or(
                Prepared::Literal(LetterReading::PreserveWords),
                Prepared::Number,
            ),
        )
    }

    pub(crate) fn temporal(source: &str, class: FallbackClass, failure: TemporalFailure) -> Self {
        match failure.preference {
            Some(preference) => Self::primary(
                source,
                class,
                failure.category,
                Prepared::Temporal(preference),
            ),
            None => Self::unresolved(source, class, failure.category),
        }
    }

    pub(crate) fn render(
        &self,
        source: &str,
        maximum: usize,
        control: &WorkControl,
    ) -> Result<(String, FallbackStrategy), NormalizeError> {
        let mut output = Output::new(maximum, control);
        let strategy = match &self.prepared {
            Prepared::Number(NumericPreference::Number(number)) => {
                output.word(&number.render().into_text())?;
                FallbackStrategy::PreferredNumber
            }
            Prepared::Number(NumericPreference::SentenceNumber(number)) => {
                output.word(&numerals::number(number).into_text())?;
                output.append(".")?;
                FallbackStrategy::PreferredNumber
            }
            Prepared::Date(date) => {
                output.word(&date.render())?;
                FallbackStrategy::SurfaceDate
            }
            Prepared::Temporal(TemporalPreference::Date(date, locative)) => {
                output.word(&verbalize::render(&Value::Date(*date, *locative)).2)?;
                FallbackStrategy::PreferredDate
            }
            Prepared::Temporal(TemporalPreference::Time(clock, locative)) => {
                output.word(&verbalize::render(&Value::Time(*clock, *locative)).2)?;
                FallbackStrategy::PreferredTime
            }
            Prepared::Time(time) => {
                output.word(&time.render())?;
                FallbackStrategy::SurfaceTime
            }
            Prepared::Literal(letters) => {
                if literal::render(
                    source,
                    *letters,
                    self.class == FallbackClass::Quantity,
                    &mut output,
                )? {
                    FallbackStrategy::UnicodeCodePoint
                } else {
                    FallbackStrategy::Literal
                }
            }
        };
        let text = output.finish();
        if text.trim().is_empty() {
            return Err(NormalizeError::Internal);
        }
        Ok((text, strategy))
    }
}

pub(crate) fn needs_reading(grapheme: &str, within_word: bool) -> bool {
    if grapheme.chars().any(|scalar| {
        spelling::symbol_name(scalar).is_some()
            && !spelling::prose_punctuation(scalar)
            && !(within_word && scalar.is_alphabetic())
    }) {
        return true;
    }
    let has_letter = grapheme.chars().any(char::is_alphabetic);
    grapheme.chars().any(|scalar| {
        !scalar.is_alphabetic()
            && !(has_letter && unicode_normalization::char::is_combining_mark(scalar))
            && !scalar.is_whitespace()
            && !spelling::prose_punctuation(scalar)
    })
}

pub(crate) fn emoticon_length(source: &str) -> Option<usize> {
    [":D", ":)", ":(", ";)", ":P", "<3"]
        .iter()
        .find(|emoticon| source.starts_with(**emoticon))
        .map(|emoticon| emoticon.len())
}
