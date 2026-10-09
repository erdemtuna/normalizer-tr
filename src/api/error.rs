//! Explicit normalization and resource failures.
use super::Issue;
use std::fmt;

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
