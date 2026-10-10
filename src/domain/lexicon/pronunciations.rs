//! Exact approved speech aliases; unknown words are never inferred.
mod ai;
mod consumer;
mod developer;

use super::{Lexeme, static_index};
use crate::{
    NormalizeError,
    morphology::{Harmony, NominalInflection, Word, WordEnd},
};

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

pub(super) fn validate() -> Result<(), NormalizeError> {
    validate_entries(&ENTRIES)
}

fn validate_entries(entries: &[(&str, &Lexeme)]) -> Result<(), NormalizeError> {
    for (_, entry) in entries {
        if entry.source != entry.target
            || entry.target.text.is_empty()
            || entry.output.trim().is_empty()
            || !entry.output.ends_with(entry.target.text)
        {
            return Err(NormalizeError::InvalidConfiguration);
        }
        for form in NominalInflection::forms() {
            let suffix = form.source_suffix(entry.source);
            let expected = form.render(entry.output, entry.target);
            let mut matched = false;
            let mut consistent = true;
            NominalInflection::analyses(entry.source, &suffix, |analysis| {
                matched = true;
                consistent &= analysis.render(entry.output, entry.target) == expected;
            });
            if !matched || !consistent {
                return Err(NormalizeError::InvalidConfiguration);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_and_phrase_indexes_cover_every_exact_key() {
        assert_eq!(ENTRIES.len(), 47);
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

    #[test]
    fn initialization_validation_accepts_the_catalog_and_syncretic_forms() {
        assert_eq!(validate(), Ok(()));
        let entry = pronounced("ayfon", "ayfon", Harmony::BackRound, WordEnd::Voiced);
        assert_eq!(validate_entries(&[("Example", &entry)]), Ok(()));
    }

    #[test]
    fn initialization_validation_rejects_inconsistent_definitions() {
        let canonical = Word::new("ti", Harmony::FrontFlat, WordEnd::Vowel);
        for entry in [
            Lexeme::distinct(
                "ti",
                Word::new("ayfon", Harmony::BackRound, WordEnd::Voiced),
                canonical,
            ),
            pronounced("", "ti", Harmony::FrontFlat, WordEnd::Vowel),
            pronounced("ti", "", Harmony::FrontFlat, WordEnd::Vowel),
            pronounced("different", "ti", Harmony::FrontFlat, WordEnd::Vowel),
        ] {
            assert_eq!(
                validate_entries(&[("Example", &entry)]),
                Err(NormalizeError::InvalidConfiguration)
            );
        }
    }
}
