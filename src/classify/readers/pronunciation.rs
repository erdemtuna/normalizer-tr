//! Bounded exact name/phrase claims; pronunciation data and suffixes stay in domain.
use super::{Attempt, Context};
use crate::{
    domain::{electronic, lexicon},
    interpretation::Value,
};

pub(super) fn read(ctx: &Context<'_>, index: usize) -> Option<Attempt> {
    let start = ctx.tokens[index];
    let first = start.text.split(['\'', '’']).next().unwrap_or(start.text);
    for &(key, entry) in lexicon::pronunciation_candidates(first).iter().rev() {
        let end = index + key.bytes().filter(|byte| *byte == b' ').count();
        let Some(last) = ctx.tokens.get(end) else {
            continue;
        };
        if electronic::looks_like(last.text) {
            continue;
        }
        let source = &ctx.text[start.range.start..last.range.end];
        if source.split(['\'', '’']).next() != Some(key) {
            continue;
        }
        return Some((
            lexicon::pronunciation_reading(*entry, source)
                .map(|(entry, case)| Value::Pronunciation(entry, case)),
            end,
        ));
    }
    None
}
