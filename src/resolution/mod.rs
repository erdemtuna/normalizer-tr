//! Static policy selection; aggregate errors and budgets stay in the pipeline.
mod fallback;

use crate::{
    AmbiguityPolicy,
    interpretation::{Reading, UnresolvedFinding, Value},
};
pub(crate) use fallback::render as render_fallback;

pub(crate) enum Selected<'a> {
    Primary(&'a Value),
    Preserved(&'a UnresolvedFinding),
    Fallback(&'a UnresolvedFinding),
}

pub(crate) fn select(reading: &Reading, policy: AmbiguityPolicy) -> Selected<'_> {
    match reading {
        Reading::Resolved(value) => Selected::Primary(value),
        Reading::Unresolved(finding) => match policy {
            AmbiguityPolicy::Preserve | AmbiguityPolicy::Reject => Selected::Preserved(finding),
            AmbiguityPolicy::Fallback => Selected::Fallback(finding),
        },
    }
}
