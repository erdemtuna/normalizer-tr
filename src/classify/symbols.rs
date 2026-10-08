use crate::interpretation::UnresolvedFinding;
use unicode_segmentation::UnicodeSegmentation;

use super::{Candidate, Reading};
use crate::{
    LimitKind, MAX_CANDIDATES, NormalizeError, SourceRange, WorkControl, source_map::SourceMap,
};

/// Supplement only unclaimed source; primary whole-expression ownership wins.
pub(crate) fn supplement(
    source: &SourceMap<'_>,
    candidates: &mut Vec<Candidate>,
    control: &WorkControl,
) -> Result<(), NormalizeError> {
    let mut added = Vec::new();
    let mut start = 0;
    for end in candidates
        .iter()
        .map(|candidate| candidate.range)
        .chain(std::iter::once(SourceRange::new(
            source.text().len(),
            source.text().len(),
        )))
    {
        scan_gap(
            source,
            start,
            end.start,
            &mut added,
            candidates.len(),
            control,
        )?;
        start = end.end;
    }
    candidates.extend(added);
    candidates.sort_by_key(|candidate| candidate.range.start);
    Ok(())
}

fn scan_gap(
    source: &SourceMap<'_>,
    start: usize,
    end: usize,
    added: &mut Vec<Candidate>,
    primary_count: usize,
    control: &WorkControl,
) -> Result<(), NormalizeError> {
    let mut cursor = start;
    let mut open = None;
    while cursor < end {
        control.check()?;
        let remaining = &source.text()[cursor..end];
        let grapheme = remaining
            .graphemes(true)
            .next()
            .ok_or(NormalizeError::Internal)?;
        let emoticon = crate::notation::emoticon_length(remaining);
        let length = emoticon.map_or(grapheme.len(), |length| {
            source.cover(SourceRange::new(cursor, cursor + length)).end - cursor
        });
        if length > end - cursor {
            return Err(NormalizeError::Internal);
        }
        let within_word = source.text()[..cursor]
            .chars()
            .next_back()
            .is_some_and(char::is_alphabetic)
            || source.text()[cursor + grapheme.len()..]
                .chars()
                .next()
                .is_some_and(char::is_alphabetic);
        if emoticon.is_some() || crate::notation::needs_reading(grapheme, within_word) {
            open.get_or_insert(cursor);
        } else if let Some(begin) = open.take() {
            append(added, SourceRange::new(begin, cursor), primary_count)?;
        }
        cursor += length;
    }
    if let Some(begin) = open {
        append(added, SourceRange::new(begin, end), primary_count)?;
    }
    Ok(())
}

fn append(
    added: &mut Vec<Candidate>,
    range: SourceRange,
    primary_count: usize,
) -> Result<(), NormalizeError> {
    if primary_count + added.len() >= MAX_CANDIDATES {
        return Err(NormalizeError::LimitExceeded(LimitKind::Candidates));
    }
    added.push(Candidate {
        range,
        reading: Reading::Unresolved(UnresolvedFinding::symbols()),
    });
    Ok(())
}
