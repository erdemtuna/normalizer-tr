use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

/// Original-source half-open UTF-8 byte range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl SourceRange {
    /// Construct coordinates. Validity against text is checked by normalization.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    /// Inclusive start byte offset.
    pub const fn start(self) -> usize {
        self.start
    }
    /// Exclusive end byte offset.
    pub const fn end(self) -> usize {
        self.end
    }
}

/// How unresolved linguistic expressions are handled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AmbiguityPolicy {
    /// Preserve source spans and return structured issues.
    #[default]
    Preserve,
    /// Return an error containing unresolved diagnostics, without partial output.
    Reject,
    /// Render unresolved source faithfully and report handled fallback diagnostics.
    Fallback,
}

/// Explicit interpretation of a whole original-source span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HintKind {
    /// An exact integer, including valid grouping and explicit zero-padding intent.
    Cardinal,
    /// ASCII digits, optional initial plus, and space/dot/slash/hyphen/parentheses.
    Digits,
    /// A valid numeric Gregorian date.
    Date,
    /// A valid 24-hour clock.
    Time,
    /// Explicit ordinal intent on a numeric period or ordinal suffix expression.
    Ordinal,
    /// Canonical uppercase Roman numeral; period establishes ordinal intent.
    Roman,
    /// Exact bounded numeric endpoints with optional quantity context.
    Range,
    /// Turkish national or +90 telephone reading.
    Telephone,
    /// Supported whole ASCII address or cued/hinted bare domain.
    Electronic,
}

/// Caller intent at original grapheme-safe byte coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hint {
    pub(crate) range: SourceRange,
    pub(crate) kind: HintKind,
}

impl Hint {
    /// Create a hint; normalization validates coordinates, overlap, and content.
    pub const fn new(range: SourceRange, kind: HintKind) -> Self {
        Self { range, kind }
    }
    /// Original-source range.
    pub const fn range(self) -> SourceRange {
        self.range
    }
    /// Requested interpretation.
    pub const fn kind(self) -> HintKind {
        self.kind
    }
}

/// Per-call options. Source-faithful fallback is opt-in.
#[derive(Clone, Debug, Default)]
pub struct NormalizeOptions {
    /// Preserve by default; explicitly reject or apply source-faithful fallback.
    pub ambiguity_policy: AmbiguityPolicy,
    /// Non-overlapping, whole-expression hints in original-source coordinates.
    pub hints: Vec<Hint>,
}

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
}

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

/// Resource limit which was exceeded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitKind {
    /// Original UTF-8 input length.
    Input,
    /// Hint count.
    Hints,
    /// Candidate work records.
    Candidates,
    /// Logical owned result allocation.
    Result,
}

/// Explicit input, policy, resource, control, or engine failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizeError {
    /// Empty/all-whitespace input or unsupported control/Bidi_Control characters.
    InvalidInput,
    /// Invalid hint range, content, overlap, or protected-expression boundary.
    InvalidHint,
    /// Bundled assets or project-owned patterns are inconsistent.
    InvalidConfiguration,
    /// A documented resource limit was exceeded.
    LimitExceeded(LimitKind),
    /// Cooperative cancellation or monotonic deadline.
    Cancelled,
    /// Strict-mode unresolved linguistic diagnostics.
    Unresolved(Vec<Issue>),
    /// Unexpected violated internal invariant.
    Internal,
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidInput => "input is empty, whitespace-only, or contains forbidden controls",
            Self::InvalidHint => "hint is not a valid whole-expression original-source range",
            Self::InvalidConfiguration => "built-in normalizer configuration is invalid",
            Self::LimitExceeded(_) => "normalization resource limit exceeded",
            Self::Cancelled => "normalization was cancelled or its deadline expired",
            Self::Unresolved(_) => "strict normalization contains unresolved linguistic work",
            Self::Internal => "normalization invariant failed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for NormalizeError {}

/// Runtime-neutral cooperative control. Clones share the cancellation signal.
#[derive(Clone, Debug, Default)]
pub struct WorkControl {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl WorkControl {
    /// Construct control with an optional monotonic deadline.
    pub fn new(deadline: Option<Instant>) -> Self {
        Self {
            deadline,
            ..Self::default()
        }
    }
    /// Cancel this control and all its clones.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub(crate) fn check(&self) -> Result<(), NormalizeError> {
        if self.cancelled.load(Ordering::Relaxed)
            || self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            Err(NormalizeError::Cancelled)
        } else {
            Ok(())
        }
    }
}
