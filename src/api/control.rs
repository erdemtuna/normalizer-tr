//! Cooperative cancellation and monotonic deadlines.
use super::NormalizeError;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

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
