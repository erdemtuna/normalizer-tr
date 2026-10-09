//! Immutable result records and their existing serialization contract.
use super::{FallbackDiagnostic, Issue, SourceRange};

/// Semantic kind of a result segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SegmentKind {
    /// Unchanged ordinary words, whitespace, or punctuation.
    Verbatim,
    /// Preserved unresolved linguistic work.
    Unresolved,
    /// Exact integer.
    Cardinal,
    /// Integer ordinal.
    Ordinal,
    /// Exact comma decimal.
    Decimal,
    /// Individual digits.
    Digits,
    /// Prefix percentage.
    Percent,
    /// Exact approved currency major/minor amount (TRY/USD/EUR/GBP).
    Money,
    /// Approved unit quantity or bounded rate.
    Unit,
    /// Gregorian date.
    Date,
    /// 24-hour digital clock.
    Time,
    /// Approved abbreviation.
    Abbreviation,
    /// Contextual or explicitly hinted numerical range.
    Range,
    /// Grouped Turkish telephone expression.
    Telephone,
    /// Full checksum-valid Turkish IBAN.
    Iban,
    /// Canonical uppercase Roman numeral with explicit/contextual intent.
    Roman,
    /// Supported email or web address.
    Electronic,
    /// Approved prose hashtag or ampersand.
    Symbol,
    /// Source-faithful fallback, not certification of the source value.
    Fallback,
    /// Approved foreign-name spoken alias with validated inflection.
    Pronunciation,
}

/// One member of an ordered, contiguous original-source partition.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Segment {
    pub(crate) range: SourceRange,
    pub(crate) kind: SegmentKind,
    pub(crate) text: String,
    pub(crate) rule_id: &'static str,
}

impl Segment {
    /// Original-source range.
    pub const fn range(&self) -> SourceRange {
        self.range
    }
    /// Semantic kind.
    pub const fn kind(&self) -> SegmentKind {
        self.kind
    }
    /// Emitted text.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Diagnostic identifier of the reader that emitted this segment.
    pub const fn rule_id(&self) -> &'static str {
        self.rule_id
    }
}

/// Owned immutable result. Completeness concerns TN work, not voice quality.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizeResult {
    pub(crate) normalized_text: String,
    pub(crate) locale: &'static str,
    pub(crate) normalizer_id: &'static str,
    pub(crate) complete: bool,
    pub(crate) segments: Vec<Segment>,
    pub(crate) issues: Vec<Issue>,
    pub(crate) fallbacks: Vec<FallbackDiagnostic>,
}

impl NormalizeResult {
    /// Concatenation of all emitted segment text.
    pub fn normalized_text(&self) -> &str {
        &self.normalized_text
    }
    /// Selected locale.
    pub const fn locale(&self) -> &'static str {
        self.locale
    }
    /// Diagnostic identity of the normalizer build.
    pub const fn normalizer_id(&self) -> &'static str {
        self.normalizer_id
    }
    /// Whether there are no unresolved TN issues.
    pub fn complete(&self) -> bool {
        self.complete
    }
    /// Ordered original-source partition.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    /// Ordered unresolved diagnostics.
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }
    /// Handled source-reading assumptions, separate from unresolved issues.
    pub fn fallbacks(&self) -> &[FallbackDiagnostic] {
        &self.fallbacks
    }
    /// Whether a fallback reading was used; derived from the diagnostic collection.
    pub fn fallback_used(&self) -> bool {
        !self.fallbacks.is_empty()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for NormalizeResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut result = serializer.serialize_struct("NormalizeResult", 8)?;
        result.serialize_field("normalized_text", &self.normalized_text)?;
        result.serialize_field("locale", &self.locale)?;
        result.serialize_field("normalizer_id", &self.normalizer_id)?;
        result.serialize_field("complete", &self.complete)?;
        result.serialize_field("segments", &self.segments)?;
        result.serialize_field("issues", &self.issues)?;
        result.serialize_field("fallbacks", &self.fallbacks)?;
        result.serialize_field("fallback_used", &self.fallback_used())?;
        result.end()
    }
}
