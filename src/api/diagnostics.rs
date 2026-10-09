//! Unresolved issues and handled-source provenance.
use super::SourceRange;

/// Source family attempted before fallback rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FallbackClass {
    /// Exact numeric notation.
    Number,
    /// Written date components, which may not form a valid calendar date.
    Date,
    /// Written clock components.
    Time,
    /// Percentage notation.
    Percent,
    /// Currency, measurement or rate notation.
    Quantity,
    /// Unapproved abbreviation.
    Abbreviation,
    /// Identifier-like source, including phone/account forms.
    Identifier,
    /// Roman-looking letters without established numeral intent.
    Roman,
    /// Address-like source.
    Electronic,
    /// Other structured notation, including ambiguous operators.
    Expression,
    /// Otherwise-unhandled symbolic graphemes.
    Symbol,
    /// Approved foreign-name family with an unsupported or invalid source form.
    Pronunciation,
}

/// Why a primary reading was unavailable; contains no copied source values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FallbackReason {
    /// Insufficient source reading intent or context.
    MissingIntent,
    /// Significant leading zeroes require a faithful digit reading.
    LeadingZeroes,
    /// A supported family rejected source grammar or logical value.
    InvalidForm,
    /// An identifier has no approved normal reading.
    ProtectedIdentifier,
    /// A source form is outside the normal grammar.
    UnsupportedForm,
    /// No approved abbreviation expansion exists.
    UnapprovedAbbreviation,
    /// A grapheme otherwise would remain unhandled.
    UnhandledSymbol,
}

/// Applied source-faithful rendering strategy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FallbackStrategy {
    /// A validated number or neutral sentence-final numeric period.
    PreferredNumber,
    /// A validated date, adopting its written date format without an explicit cue.
    PreferredDate,
    /// A validated clock, adopting its written clock format without an explicit cue.
    PreferredTime,
    /// Date-shaped source components without calendar certification.
    SurfaceDate,
    /// Clock-shaped source components without clock certification.
    SurfaceTime,
    /// Ordered literal words, letters, digits and symbol names.
    Literal,
    /// One or more unnamed scalars spoken as conventional hexadecimal U+ codes.
    UnicodeCodePoint,
}

/// Immutable provenance for one handled original-source fallback span.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FallbackDiagnostic {
    pub(crate) range: SourceRange,
    pub(crate) attempted_class: FallbackClass,
    pub(crate) reason: FallbackReason,
    pub(crate) original_category: Option<IssueCategory>,
    pub(crate) strategy: FallbackStrategy,
}

impl FallbackDiagnostic {
    /// Original-source half-open UTF-8 range.
    pub const fn range(&self) -> SourceRange {
        self.range
    }
    /// Source family, not a claim that its value was valid.
    pub const fn attempted_class(&self) -> FallbackClass {
        self.attempted_class
    }
    /// Primary-reading reason or uncovered-symbol reason.
    pub const fn reason(&self) -> FallbackReason {
        self.reason
    }
    /// Original primary issue category, absent for newly covered symbols.
    pub const fn original_category(&self) -> Option<IssueCategory> {
        self.original_category
    }
    /// Strategy which completed the reading.
    pub const fn strategy(&self) -> FallbackStrategy {
        self.strategy
    }
}

/// Machine-readable reason for preserved linguistic work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum IssueCategory {
    /// Multiple or insufficiently cued readings.
    Ambiguous,
    /// Invalid supported grammar, value, or suffix allomorph.
    InvalidExpression,
    /// Structured identifier that must not be rewritten in fragments.
    ProtectedIdentifier,
    /// Expression outside the normalizer's bounded coverage.
    Unsupported,
    /// Unapproved uppercase abbreviation.
    UnknownAbbreviation,
}

/// Non-sensitive diagnostic for an unresolved original-source range.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Issue {
    pub(crate) range: SourceRange,
    pub(crate) category: IssueCategory,
    pub(crate) explanation: &'static str,
}

impl Issue {
    /// Original-source range.
    pub const fn range(&self) -> SourceRange {
        self.range
    }
    /// Machine-readable category.
    pub const fn category(&self) -> IssueCategory {
        self.category
    }
    /// Static explanation; contains no copied input values.
    pub const fn explanation(&self) -> &'static str {
        self.explanation
    }
}
