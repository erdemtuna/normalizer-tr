use super::scan::{CatalogGroup, Token, overlaps};
use crate::{Hint, HintKind, NormalizeError, SourceRange, notation};

/// The one source of cursor advancement, source claims and whole-hint ownership.
pub(super) struct Boundaries<'a> {
    pub(super) tokens: &'a [Token<'a>],
    phone_like: &'a [SourceRange],
    catalog_groups: &'a [CatalogGroup],
}

pub(super) struct Claim {
    pub(super) range: SourceRange,
    pub(super) next: usize,
}

impl<'a> Boundaries<'a> {
    pub(super) fn new(
        tokens: &'a [Token<'a>],
        phone_like: &'a [SourceRange],
        catalog_groups: &'a [CatalogGroup],
    ) -> Self {
        Self {
            tokens,
            phone_like,
            catalog_groups,
        }
    }

    pub(super) fn catalog_allowed(&self, start: usize, end: usize) -> bool {
        let range = SourceRange::new(self.tokens[start].range.start, self.tokens[end].range.end);
        let index = self
            .catalog_groups
            .partition_point(|group| group.range.end <= range.start);
        !self.catalog_groups[index..]
            .iter()
            .take_while(|group| group.range.start < range.end)
            .any(|group| group.catalog && !group.approved)
    }
    pub(super) fn claim(&self, start: usize, end: usize) -> Result<Claim, NormalizeError> {
        let first = self.tokens.get(start).ok_or(NormalizeError::Internal)?;
        let last = self
            .tokens
            .get(end)
            .filter(|_| end >= start)
            .ok_or(NormalizeError::Internal)?;
        Ok(Claim {
            range: SourceRange::new(first.range.start, last.range.end),
            next: end + 1,
        })
    }
    pub(super) fn claim_through(
        &self,
        start: usize,
        end: usize,
        source_end: usize,
    ) -> Result<Claim, NormalizeError> {
        let mut claim = self.claim(start, end)?;
        if source_end < claim.range.end {
            return Err(NormalizeError::Internal);
        }
        if source_end > claim.range.end {
            let next = self.next_at(source_end);
            if next
                .checked_sub(1)
                .and_then(|index| self.tokens.get(index))
                .is_some_and(|token| token.range.end > source_end)
            {
                return Err(NormalizeError::Internal);
            }
            claim.range.end = source_end;
            claim.next = next;
        }
        Ok(claim)
    }
    pub(super) fn hint_claim(
        &self,
        hint: Hint,
        detected: Option<&Claim>,
    ) -> Result<Claim, NormalizeError> {
        let range = hint.range;
        let start = self.tokens.partition_point(|t| t.range.end <= range.start);
        let end = self.tokens.partition_point(|t| t.range.start < range.end);
        let touched = &self.tokens[start..end];
        let wrap_allowed = matches!(hint.kind, HintKind::Digits | HintKind::Telephone);
        let phones = &self.phone_like[self.phone_like.partition_point(|p| p.end <= range.start)
            ..self.phone_like.partition_point(|p| p.start < range.end)];
        if touched.is_empty()
            || (!wrap_allowed
                && (touched.first().is_none_or(|t| t.range.start != range.start)
                    || touched.last().is_none_or(|t| t.range.end != range.end)))
            || touched
                .iter()
                .any(|t| t.range.start < range.start || t.range.end > range.end)
            || phones
                .iter()
                .any(|p| overlaps(*p, range) && (*p != range || !wrap_allowed))
        {
            return Err(NormalizeError::InvalidHint);
        }
        if detected.is_some_and(|span| span.range.start < range.start || span.range.end > range.end)
        {
            return Err(NormalizeError::InvalidHint);
        }
        Ok(Claim { range, next: end })
    }
    pub(super) fn phone_at(&self, start: usize) -> Option<SourceRange> {
        self.phone_like
            .binary_search_by_key(&start, |p| p.start)
            .ok()
            .map(|i| self.phone_like[i])
    }
    pub(super) fn next_at(&self, end: usize) -> usize {
        self.tokens.partition_point(|t| t.range.start < end)
    }
}

pub(super) fn overlaps_hint(hints: &[Hint], range: SourceRange) -> bool {
    hints
        .get(hints.partition_point(|h| h.range.end <= range.start))
        .is_some_and(|h| overlaps(h.range, range))
}

pub(super) fn quantity_tail(text: &str) -> bool {
    if text.starts_with(['\'', '’']) {
        return true;
    }
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    notation::label(base)
        || notation::unsupported_label(base)
        || notation::unsupported_label(&base.to_ascii_uppercase())
        || notation::currency_marker(base)
        || crate::domain::lexicon::unit_marker(base)
        || base == "%"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_extended_claim_cannot_discard_a_partially_touched_token() {
        let tokens = [
            Token {
                range: SourceRange::new(0, 1),
                text: "1",
                number_run_end: 0,
                phone_run_end: 0,
                digit_end: 1,
                standalone: false,
            },
            Token {
                range: SourceRange::new(2, 6),
                text: "AB12",
                number_run_end: 1,
                phone_run_end: 1,
                digit_end: 3,
                standalone: false,
            },
        ];
        let bounds = Boundaries::new(&tokens, &[], &[]);
        assert!(matches!(
            bounds.claim_through(0, 0, 3),
            Err(NormalizeError::Internal)
        ));
        let claim = bounds.claim_through(0, 0, 6).unwrap();
        assert_eq!(claim.range, SourceRange::new(0, 6));
        assert_eq!(claim.next, 2);
    }
}
