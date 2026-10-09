//! Public original-source coordinates.

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
