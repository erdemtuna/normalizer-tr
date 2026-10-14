mod addresses;
mod boundaries;
mod context;
mod readers;
mod scan;
mod symbols;
mod temporal;

use crate::{
    Hint, LimitKind, MAX_CANDIDATES, NormalizeError, SourceRange, WorkControl,
    resources::Resources, source_map::SourceMap,
};
use boundaries::{Boundaries, Claim, overlaps_hint};
use readers::Context;
pub(crate) use symbols::supplement;

pub(crate) struct Candidate {
    pub(crate) range: SourceRange,
    pub(crate) reading: Reading,
}

pub(crate) use crate::interpretation::Reading;

fn push(
    candidates: &mut Vec<Candidate>,
    claim: Claim,
    reading: Reading,
) -> Result<(), NormalizeError> {
    if candidates.len() == MAX_CANDIDATES {
        return Err(NormalizeError::LimitExceeded(LimitKind::Candidates));
    }
    candidates.push(Candidate {
        range: claim.range,
        reading,
    });
    Ok(())
}

pub(crate) fn collect(
    source: &SourceMap,
    hints: &[Hint],
    resources: &Resources,
    control: &WorkControl,
) -> Result<Vec<Candidate>, NormalizeError> {
    let text = source.text();
    let scanned = scan::tokens(source, resources, control)?;
    let tokens = scanned.tokens;
    let address_fields = addresses::collect(text, control)?;
    let phone_like = scan::phones(text, &tokens, resources, control)?;
    let bounds = Boundaries::new(&tokens, &phone_like, &scanned.catalog_groups);
    let mut address_fields = address_fields
        .into_iter()
        .map(|field| {
            let start = bounds.next_at(field.range.start);
            (field, start)
        })
        .peekable();
    let mut candidates = Vec::new();
    let mut index = 0;
    let mut hint_index = 0;
    while index < tokens.len() {
        control.check()?;
        let address = if address_fields
            .peek()
            .is_some_and(|(field, _)| field.range.start == tokens[index].range.start)
        {
            address_fields.next().map(|(field, _)| field)
        } else {
            None
        };
        // Independent address owners bound lookahead before any reader is invoked.
        let end = address_fields
            .peek()
            .map_or(tokens.len(), |(_, start)| *start);
        if end <= index {
            return Err(NormalizeError::Internal);
        }
        let source_end = tokens
            .get(end)
            .map_or(text.len(), |token| token.range.start);
        let ctx = Context {
            text: &text[..source_end],
            tokens: &tokens[..end],
        };
        let detected = address.as_ref().map(|field| Claim {
            range: field.range,
            next: bounds.next_at(field.range.end),
        });
        if let Some(hint) = hints
            .get(hint_index)
            .filter(|h| h.range.start <= tokens[index].range.start)
        {
            let detected = match detected {
                Some(claim) => Some(claim),
                None => readers::read(&ctx, index, &bounds, control)?.map(|(_, claim)| claim),
            };
            let claim = bounds.hint_claim(*hint, detected.as_ref())?;
            let text = &text[claim.range.start..claim.range.end];
            let following_ile = tokens.get(claim.next).is_some_and(|next| {
                scan::whitespace_between(source.text(), claim.range.end, next.range.start)
                    && crate::domain::lexicon::lookup_key(next.text) == "arasında"
            });
            let value =
                readers::hint(text, hint.kind, following_ile).ok_or(NormalizeError::InvalidHint)?;
            index = claim.next;
            push(&mut candidates, claim, Reading::Resolved(value))?;
            hint_index += 1;
            continue;
        }
        if hints
            .get(hint_index)
            .is_some_and(|h| h.range.start < tokens[index].range.end)
        {
            return Err(NormalizeError::InvalidHint);
        }
        if let Some(field) = address {
            if overlaps_hint(hints, field.range) {
                return Err(NormalizeError::InvalidHint);
            }
            index = bounds.next_at(field.range.end);
            push(
                &mut candidates,
                Claim {
                    range: field.range,
                    next: index,
                },
                field.reading,
            )?;
        } else if let Some((reading, claim)) = readers::read(&ctx, index, &bounds, control)? {
            if overlaps_hint(hints, claim.range) {
                return Err(NormalizeError::InvalidHint);
            }
            index = claim.next;
            push(&mut candidates, claim, reading)?;
        } else {
            index += 1;
        }
    }
    if hint_index != hints.len() {
        return Err(NormalizeError::InvalidHint);
    }
    if address_fields.next().is_some() {
        return Err(NormalizeError::Internal);
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpretation::UnresolvedFinding;
    #[test]
    fn candidate_limit_accepts_exact_boundary() {
        let mut candidates = Vec::new();
        for _ in 0..MAX_CANDIDATES {
            assert!(
                push(
                    &mut candidates,
                    Claim {
                        range: SourceRange::new(0, 1),
                        next: 1
                    },
                    Reading::Unresolved(UnresolvedFinding::unresolved(
                        "1.234",
                        crate::FallbackClass::Number,
                        crate::IssueCategory::Ambiguous
                    ))
                )
                .is_ok()
            );
        }
        assert!(matches!(
            push(
                &mut candidates,
                Claim {
                    range: SourceRange::new(0, 1),
                    next: 1
                },
                Reading::Unresolved(UnresolvedFinding::unresolved(
                    "1.234",
                    crate::FallbackClass::Number,
                    crate::IssueCategory::Ambiguous
                ))
            ),
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates))
        ));
    }
}
