use std::mem::size_of;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AmbiguityPolicy, FallbackDiagnostic, Issue, IssueCategory, LimitKind, MAX_HINTS,
    MAX_INPUT_BYTES, MAX_RESULT_BYTES, NormalizeError, NormalizeOptions, NormalizeResult, Segment,
    SegmentKind, SourceRange, WorkControl, classify, resolution, resources::Resources,
    source_map::SourceMap, verbalize,
};

#[derive(Default)]
struct ResultBudget {
    used: usize,
}

impl ResultBudget {
    fn charge(&mut self, bytes: usize) -> Result<(), NormalizeError> {
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|used| *used <= MAX_RESULT_BYTES)
            .ok_or(NormalizeError::LimitExceeded(LimitKind::Result))?;
        Ok(())
    }
    fn segment(&mut self, text_bytes: usize) -> Result<(), NormalizeError> {
        // Text is owned once per segment and once again by normalized_text.
        self.charge(
            text_bytes
                .checked_mul(2)
                .and_then(|bytes| bytes.checked_add(size_of::<Segment>()))
                .ok_or(NormalizeError::LimitExceeded(LimitKind::Result))?,
        )
    }
    fn issue(&mut self) -> Result<(), NormalizeError> {
        self.charge(size_of::<Issue>())
    }
    fn text_allowance(&self) -> Result<usize, NormalizeError> {
        MAX_RESULT_BYTES
            .checked_sub(self.used)
            .and_then(|bytes| bytes.checked_sub(size_of::<Segment>()))
            .map(|bytes| bytes / 2)
            .ok_or(NormalizeError::LimitExceeded(LimitKind::Result))
    }
}

fn explanation(category: IssueCategory) -> &'static str {
    match category {
        IssueCategory::Ambiguous => "expression requires an explicit supported cue or hint",
        IssueCategory::InvalidExpression => "expression has an invalid value, grammar, or suffix",
        IssueCategory::ProtectedIdentifier => "structured identifier is preserved in full",
        IssueCategory::Unsupported => "expression is outside the bounded profile",
        IssueCategory::UnknownAbbreviation => "uppercase abbreviation is not approved",
    }
}

fn verbatim(
    input: &str,
    start: usize,
    end: usize,
    segments: &mut Vec<Segment>,
    budget: &mut ResultBudget,
) -> Result<(), NormalizeError> {
    if start != end {
        budget.segment(end - start)?;
        segments.push(Segment {
            range: SourceRange::new(start, end),
            kind: SegmentKind::Verbatim,
            text: input[start..end].to_owned(),
            rule_id: "source.verbatim",
        });
    }
    Ok(())
}

fn fallback_reading(
    request: &crate::interpretation::UnresolvedFinding,
    input: &str,
    range: SourceRange,
    recognition: &str,
    budget: &mut ResultBudget,
    control: &WorkControl,
) -> Result<(String, FallbackDiagnostic), NormalizeError> {
    budget.charge(size_of::<FallbackDiagnostic>())?;
    let leading_space = input[..range.start]
        .graphemes(true)
        .next_back()
        .is_some_and(|grapheme| grapheme.chars().any(char::is_alphanumeric));
    let trailing_space = input[range.end..]
        .graphemes(true)
        .next()
        .is_some_and(|grapheme| grapheme.chars().any(char::is_alphanumeric));
    let padding = usize::from(leading_space) + usize::from(trailing_space);
    let maximum = budget
        .text_allowance()?
        .checked_sub(padding)
        .ok_or(NormalizeError::LimitExceeded(LimitKind::Result))?;
    let (mut text, strategy) = resolution::render_fallback(request, recognition, maximum, control)?;
    if leading_space {
        text.insert(0, ' ');
    }
    if trailing_space {
        text.push(' ');
    }
    Ok((
        text,
        FallbackDiagnostic {
            range,
            attempted_class: request.class(),
            reason: request.reason(),
            original_category: request.category(),
            strategy,
        },
    ))
}

pub(crate) fn run(
    input: &str,
    options: &NormalizeOptions,
    control: &WorkControl,
    rules: &Resources,
) -> Result<NormalizeResult, NormalizeError> {
    control.check()?;
    if input.len() > MAX_INPUT_BYTES {
        return Err(NormalizeError::LimitExceeded(LimitKind::Input));
    }
    if options.hints.len() > MAX_HINTS {
        return Err(NormalizeError::LimitExceeded(LimitKind::Hints));
    }
    if input.trim().is_empty()
        || input.chars().any(|c| {
            (c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
    {
        return Err(NormalizeError::InvalidInput);
    }
    let source = SourceMap::new(input, control)?;
    let hints = source.hints(&options.hints)?;
    let mut candidates = classify::collect(&source, &hints, rules, control)?;
    if options.ambiguity_policy == AmbiguityPolicy::Fallback {
        classify::supplement(&source, &mut candidates, control)?;
    }
    let mut budget = ResultBudget::default();
    budget.charge(size_of::<NormalizeResult>())?;
    let mut segments = Vec::new();
    let mut issues = Vec::new();
    let mut fallbacks = Vec::new();
    let mut cursor = 0;
    for candidate in candidates {
        control.check()?;
        let range = source
            .original(candidate.range)
            .map_err(|_| NormalizeError::Internal)?;
        if range.start < cursor || range.start >= range.end || range.end > input.len() {
            return Err(NormalizeError::Internal);
        }
        verbatim(input, cursor, range.start, &mut segments, &mut budget)?;
        let (kind, rule_id, text) =
            match resolution::select(&candidate.reading, options.ambiguity_policy) {
                resolution::Selected::Primary(value) => verbalize::render(value),
                resolution::Selected::Fallback(request) => {
                    let (text, diagnostic) = fallback_reading(
                        request,
                        input,
                        range,
                        &source.text()[candidate.range.start..candidate.range.end],
                        &mut budget,
                        control,
                    )?;
                    fallbacks.push(diagnostic);
                    (SegmentKind::Fallback, "source.fallback", text)
                }
                resolution::Selected::Preserved(request) => {
                    let category = request.category().ok_or(NormalizeError::Internal)?;
                    budget.issue()?;
                    issues.push(Issue {
                        range,
                        category,
                        explanation: explanation(category),
                    });
                    (
                        SegmentKind::Unresolved,
                        "source.unresolved",
                        input[range.start..range.end].to_owned(),
                    )
                }
            };
        budget.segment(text.len())?;
        segments.push(Segment {
            range,
            kind,
            text,
            rule_id,
        });
        cursor = range.end;
    }
    verbatim(input, cursor, input.len(), &mut segments, &mut budget)?;
    control.check()?;
    if options.ambiguity_policy == AmbiguityPolicy::Reject && !issues.is_empty() {
        return Err(NormalizeError::Unresolved(issues));
    }
    let length = segments.iter().map(|segment| segment.text.len()).sum();
    let mut normalized_text = String::with_capacity(length);
    for segment in &segments {
        control.check()?;
        normalized_text.push_str(&segment.text);
    }
    Ok(NormalizeResult {
        normalized_text,
        locale: "tr-TR",
        normalizer_id: crate::NORMALIZER_ID,
        complete: issues.is_empty(),
        segments,
        issues,
        fallbacks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_budget_exact_boundary_and_overflow() {
        let mut budget = ResultBudget::default();
        assert_eq!(budget.charge(MAX_RESULT_BYTES), Ok(()));
        assert_eq!(
            budget.charge(1),
            Err(NormalizeError::LimitExceeded(LimitKind::Result))
        );
        let mut budget = ResultBudget::default();
        assert_eq!(
            budget.charge(usize::MAX),
            Err(NormalizeError::LimitExceeded(LimitKind::Result))
        );
    }
}
