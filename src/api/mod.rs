//! Public contract facade; crate-root import paths remain unchanged.
mod control;
mod diagnostics;
mod error;
mod options;
mod result;
mod source;

pub use control::WorkControl;
pub use diagnostics::{
    FallbackClass, FallbackDiagnostic, FallbackReason, FallbackStrategy, Issue, IssueCategory,
};
pub use error::{LimitKind, NormalizeError};
pub use options::{AmbiguityPolicy, Hint, HintKind, NormalizeOptions};
pub use result::{NormalizeResult, Segment, SegmentKind};
pub use source::SourceRange;
