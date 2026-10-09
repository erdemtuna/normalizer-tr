//! Exact approved speech aliases; unknown words are never inferred.
mod ai;
mod consumer;
mod developer;

use super::{Lexeme, static_index};
use crate::morphology::{Harmony, Word, WordEnd};

const fn pronounced(
    output: &'static str,
    tail: &'static str,
    harmony: Harmony,
    end: WordEnd,
) -> Lexeme {
    let word = Word::new(tail, harmony, end);
    Lexeme::distinct(output, word, word)
}

const GROUPS: &[&[(&str, Lexeme)]] = &[ai::ENTRIES, developer::ENTRIES, consumer::ENTRIES];
const ENTRIES: [(&str, &Lexeme); static_index::count(GROUPS)] = static_index::build(GROUPS);
const MAX_KEY_BYTES: usize = static_index::maximum_key_bytes(&ENTRIES);
const FIRST_BYTES: [bool; u8::MAX as usize + 1] = {
    let mut bytes = [false; u8::MAX as usize + 1];
    let mut entry = 0;
    while entry < ENTRIES.len() {
        bytes[ENTRIES[entry].0.as_bytes()[0] as usize] = true;
        entry += 1;
    }
    bytes
};

pub(super) fn lookup(key: &str) -> Option<Lexeme> {
    if !possible_key(key) {
        return None;
    }
    ENTRIES
        .binary_search_by(|(candidate, _)| candidate.cmp(&key))
        .ok()
        .map(|index| *ENTRIES[index].1)
}

pub(super) fn candidates(first: &str) -> &'static [(&'static str, &'static Lexeme)] {
    if !possible_key(first) {
        return &[];
    }
    let start = ENTRIES.partition_point(|(key, _)| first_word(key) < first);
    let end = ENTRIES.partition_point(|(key, _)| first_word(key) <= first);
    &ENTRIES[start..end]
}

fn possible_key(text: &str) -> bool {
    text.len() <= MAX_KEY_BYTES
        && text
            .as_bytes()
            .first()
            .is_some_and(|byte| FIRST_BYTES[usize::from(*byte)])
}

pub(super) fn boundary_word(text: &str) -> bool {
    let base = text.split(['\'', '’']).next().unwrap_or(text);
    if base.len() > MAX_KEY_BYTES {
        return false;
    }
    lookup(base).is_some()
        || ENTRIES.iter().any(|(key, _)| {
            key.contains(' ') && key.rsplit(' ').next().is_some_and(|last| last == base)
        })
}

fn first_word(key: &str) -> &str {
    key.split(' ').next().unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_and_phrase_indexes_cover_every_exact_key() {
        assert_eq!(ENTRIES.len(), 44);
        for (key, entry) in &ENTRIES {
            assert_eq!(lookup(key).unwrap().output, entry.output);
            assert!(
                candidates(first_word(key))
                    .iter()
                    .any(|(candidate, _)| candidate == key)
            );
            assert!(boundary_word(key.rsplit(' ').next().unwrap()));
            assert!(super::super::abbreviation(key).is_none(), "{key}");
        }
        assert!(
            candidates("GitHub")
                .iter()
                .any(|(key, _)| *key == "GitHub Copilot")
        );
        for unknown in [
            "apple", "codex", "react", "rust", "python", "Face", "Studio", "Code",
        ] {
            assert!(lookup(unknown).is_none(), "{unknown}");
        }
    }
}
