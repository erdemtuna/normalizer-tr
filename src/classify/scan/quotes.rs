//! Certified quote framing shared by tokenization and raw atom admission.
use super::punctuation::quotation_boundaries;
use crate::{NormalizeError, WorkControl};

#[derive(Clone, Copy)]
enum QuoteKind {
    StraightSingle,
    StraightDouble,
    SmartSingle,
    SmartDouble,
}

impl QuoteKind {
    const COUNT: usize = 4;

    fn parse(ch: char) -> Option<Self> {
        match ch {
            '\'' => Some(Self::StraightSingle),
            '"' => Some(Self::StraightDouble),
            '‘' => Some(Self::SmartSingle),
            '“' => Some(Self::SmartDouble),
            _ => None,
        }
    }

    fn opening(self) -> char {
        match self {
            Self::StraightSingle => '\'',
            Self::StraightDouble => '"',
            Self::SmartSingle => '‘',
            Self::SmartDouble => '“',
        }
    }

    fn closing(self) -> char {
        match self {
            Self::StraightSingle => '\'',
            Self::StraightDouble => '"',
            Self::SmartSingle => '’',
            Self::SmartDouble => '”',
        }
    }
}

struct OpenQuote {
    start: usize,
    kind: QuoteKind,
}

impl OpenQuote {
    fn closes(&self, ch: char) -> bool {
        self.kind.closing() == ch
    }

    fn immediately_precedes(&self, offset: usize) -> bool {
        self.start + self.kind.opening().len_utf8() == offset
    }
}

pub(super) struct QuoteBoundaries {
    legacy_pairs: Vec<usize>,
    membership: Option<Vec<usize>>,
    transparent: Vec<bool>,
}

impl QuoteBoundaries {
    pub(super) fn collect(text: &str, control: &WorkControl) -> Result<Self, NormalizeError> {
        let transparent = certify(text, control)?;
        let legacy_pairs = quotation_boundaries(text);
        let membership = if transparent.is_empty() {
            None
        } else {
            Some(merge(&legacy_pairs, &transparent, control)?)
        };
        Ok(Self {
            legacy_pairs,
            membership,
            transparent,
        })
    }

    /// Sorted boundary membership, not opening/closing pairs.
    pub(super) fn offsets(&self) -> &[usize] {
        self.membership.as_deref().unwrap_or(&self.legacy_pairs)
    }

    pub(super) fn legacy_pairs(&self) -> &[usize] {
        &self.legacy_pairs
    }

    pub(super) fn transparent(&self) -> &[bool] {
        &self.transparent
    }
}

fn certify(text: &str, control: &WorkControl) -> Result<Vec<bool>, NormalizeError> {
    if !text.contains(['\'', '"', '‘', '’', '“', '”']) {
        return Ok(Vec::new());
    }
    let mut transparent = Vec::new();
    let mut stack: Vec<OpenQuote> = Vec::new();
    let mut pending = Vec::new();
    let mut active = [0_usize; QuoteKind::COUNT];
    let mut ambiguous = false;
    let mut previous: Option<char> = None;
    let mut chars = text.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        control.check()?;
        let next = chars.peek().map(|(_, ch)| *ch);
        let parent_closes = stack.len() > 1 && next == Some(stack[stack.len() - 2].kind.closing());
        let closing_boundary = next.is_none_or(char::is_whitespace) || parent_closes;
        let suffix_apostrophe = matches!(ch, '\'' | '’')
            && previous.is_some_and(char::is_alphanumeric)
            && next.is_some_and(char::is_alphanumeric);
        if stack.last().is_some_and(|quote| quote.closes(ch)) && !suffix_apostrophe {
            ambiguous |= !closing_boundary;
            let quote = stack.pop().ok_or(NormalizeError::Internal)?;
            active[quote.kind as usize] -= 1;
            pending.push(offset);
            if stack.is_empty() {
                if !ambiguous {
                    if transparent.is_empty() {
                        transparent.resize(text.len(), false);
                    }
                    for position in pending.drain(..) {
                        control.check()?;
                        transparent[position] = true;
                    }
                } else {
                    pending.clear();
                }
                ambiguous = false;
            }
        } else if let Some(kind) = QuoteKind::parse(ch)
            && (previous.is_none_or(char::is_whitespace)
                || stack
                    .last()
                    .is_some_and(|quote| quote.immediately_precedes(offset)))
            && next.is_some()
        {
            ambiguous |= active[kind as usize] != 0;
            active[kind as usize] += 1;
            stack.push(OpenQuote {
                start: offset,
                kind,
            });
            pending.push(offset);
        } else if !stack.is_empty() && matches!(ch, '\'' | '"' | '‘' | '’' | '“' | '”') {
            ambiguous |= !suffix_apostrophe;
        }
        previous = Some(ch);
    }
    Ok(transparent)
}

fn merge(
    legacy: &[usize],
    certified: &[bool],
    control: &WorkControl,
) -> Result<Vec<usize>, NormalizeError> {
    if certified.is_empty() {
        return Ok(legacy.to_vec());
    }
    // Legacy grammar remains authoritative; certification only adds boundaries.
    let mut legacy = legacy.iter().copied().peekable();
    let mut merged = Vec::new();
    for (offset, transparent) in certified.iter().enumerate() {
        control.check()?;
        if !transparent {
            continue;
        }
        while legacy.peek().is_some_and(|previous| *previous < offset) {
            control.check()?;
            merged.push(legacy.next().ok_or(NormalizeError::Internal)?);
        }
        if legacy.peek() == Some(&offset) {
            legacy.next();
        }
        merged.push(offset);
    }
    for offset in legacy {
        control.check()?;
        merged.push(offset);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests;
