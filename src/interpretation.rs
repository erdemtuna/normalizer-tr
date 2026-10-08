//! Policy-neutral values, unresolved findings and prevalidated alternatives.
use crate::{
    FallbackClass, FallbackReason, IssueCategory,
    domain::{
        electronic::Electronic,
        identifiers::{Iban, Telephone},
        numeric::{Numeric, NumericFailure, NumericPreference},
        quantities::{NumericRange, Quantity},
        temporal::{Clock, Date},
    },
    morphology::Inflection,
    notation::{DateSurface, TimeSurface},
    numerals::Number,
};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Numeric(Numeric),
    Digits(String),
    Percent(Number, Option<Inflection>),
    Date(Date, bool),
    Time(Clock, bool),
    Quantity(Quantity),
    Lexical(crate::domain::lexicon::Lexeme, Option<Inflection>),
    Range(NumericRange),
    Telephone(Telephone),
    Iban(Iban),
    Roman(Numeric),
    Electronic(Electronic),
    Symbol(String),
}

pub(crate) enum TemporalPreference {
    Date(Date, bool),
    Time(Clock, bool),
}

pub(crate) struct TemporalFailure {
    pub(crate) category: crate::IssueCategory,
    pub(crate) preference: Option<TemporalPreference>,
}

impl From<crate::IssueCategory> for TemporalFailure {
    fn from(category: crate::IssueCategory) -> Self {
        Self {
            category,
            preference: None,
        }
    }
}

pub(crate) enum Reading {
    Resolved(Value),
    Unresolved(UnresolvedFinding),
}

#[derive(Clone, Copy)]
pub(crate) enum LiteralReading {
    Spell,
    PreserveWords,
}

pub(crate) enum Alternative {
    Number(NumericPreference),
    Temporal(TemporalPreference),
    Date(DateSurface),
    Time(TimeSurface),
    Literal(LiteralReading),
}

/// Keeps the failed family and readable source alternative together.
pub(crate) struct UnresolvedFinding {
    category: Option<IssueCategory>,
    class: FallbackClass,
    reason: FallbackReason,
    alternative: Alternative,
}

impl UnresolvedFinding {
    pub(crate) fn alternative(&self) -> &Alternative {
        &self.alternative
    }
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
        let alternative = match class {
            FallbackClass::Date => DateSurface::parse(source).map(Alternative::Date),
            FallbackClass::Time => TimeSurface::parse(source).map(Alternative::Time),
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
            alternative.unwrap_or(Alternative::Literal(if spell {
                LiteralReading::Spell
            } else {
                LiteralReading::PreserveWords
            })),
        )
    }

    fn primary(
        source: &str,
        class: FallbackClass,
        category: IssueCategory,
        alternative: Alternative,
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
            alternative,
        }
    }

    pub(crate) fn symbols() -> Self {
        Self {
            category: None,
            class: FallbackClass::Symbol,
            reason: FallbackReason::UnhandledSymbol,
            alternative: Alternative::Literal(LiteralReading::Spell),
        }
    }

    pub(crate) fn numeric(source: &str, failure: NumericFailure) -> Self {
        Self::primary(
            source,
            FallbackClass::Number,
            failure.category,
            failure.preference.map_or(
                Alternative::Literal(LiteralReading::PreserveWords),
                Alternative::Number,
            ),
        )
    }

    pub(crate) fn temporal(source: &str, class: FallbackClass, failure: TemporalFailure) -> Self {
        match failure.preference {
            Some(preference) => Self::primary(
                source,
                class,
                failure.category,
                Alternative::Temporal(preference),
            ),
            None => Self::unresolved(source, class, failure.category),
        }
    }
}
