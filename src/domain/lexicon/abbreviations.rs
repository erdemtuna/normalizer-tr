//! Approved lexical readings, separate from source traversal and policy handling.
mod civic;
mod education;
mod finance;
mod technology;
mod titles;

use super::{Lexeme, static_index};
use crate::morphology::{Harmony, Inflection, Word, WordEnd::Vowel, case_inflection};

#[derive(Clone, Copy)]
pub(super) struct Definition {
    default: Lexeme,
    alternatives: &'static [Lexeme],
}

impl Definition {
    const fn single(default: Lexeme) -> Self {
        Self::with_alternatives(default, &[])
    }

    const fn with_alternatives(default: Lexeme, alternatives: &'static [Lexeme]) -> Self {
        Self {
            default,
            alternatives,
        }
    }

    pub(super) fn default(&self) -> Lexeme {
        self.default
    }

    pub(super) fn inflected(&self, suffix: &str) -> Option<(Lexeme, Inflection)> {
        std::iter::once(&self.default)
            .chain(self.alternatives)
            .find_map(|entry| case_inflection(entry.source, suffix).map(|case| (*entry, case)))
    }
}

const fn spelled(output: &'static str, tail: &'static str, harmony: Harmony) -> Lexeme {
    let word = Word::new(tail, harmony, Vowel);
    Lexeme::distinct(output, word, word)
}

const fn initialism(output: &'static str, tail: &'static str, harmony: Harmony) -> Definition {
    Definition::single(spelled(output, tail, harmony))
}

const GROUPS: &[&[(&str, Definition)]] = &[
    titles::ENTRIES,
    civic::ENTRIES,
    education::ENTRIES,
    finance::ENTRIES,
    technology::ENTRIES,
];

const ENTRY_COUNT: usize = static_index::count(GROUPS);

// Author groups in semantic order; generate the binary-search index at compile time.
const ENTRIES: [(&str, &Definition); ENTRY_COUNT] = static_index::build(GROUPS);
const MAX_KEY_BYTES: usize = static_index::maximum_key_bytes(&ENTRIES);

pub(super) fn lookup(symbol: &str) -> Option<&'static Definition> {
    if symbol.len() > MAX_KEY_BYTES
        || (!symbol.ends_with('.') && !symbol.chars().all(char::is_uppercase))
    {
        return None;
    }
    ENTRIES
        .binary_search_by(|(key, _)| key.cmp(&symbol))
        .ok()
        .map(|index| ENTRIES[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn generated_index_covers_every_group_with_unique_sorted_keys() {
        assert_eq!(ENTRIES.len(), 80);
        assert!(ENTRIES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for group in GROUPS {
            assert!(!group.is_empty());
            for (key, definition) in *group {
                assert!(key.ends_with('.') || key.chars().all(char::is_uppercase));
                let selected = lookup(key).unwrap();
                assert_eq!(selected.default.output, definition.default.output);
                assert_eq!(
                    selected
                        .alternatives
                        .iter()
                        .map(|entry| entry.output)
                        .collect::<Vec<_>>(),
                    definition
                        .alternatives
                        .iter()
                        .map(|entry| entry.output)
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn semantic_groups_do_not_require_alphabetical_order() {
        assert!(
            GROUPS
                .iter()
                .any(|group| { group.windows(2).any(|pair| pair[0].0 > pair[1].0) })
        );
    }

    #[test]
    fn variant_suffixes_are_unambiguous() {
        for (_, definition) in &ENTRIES {
            let mut suffixes = BTreeSet::new();
            for reading in std::iter::once(&definition.default).chain(definition.alternatives) {
                for case in [
                    Inflection::Accusative,
                    Inflection::Dative,
                    Inflection::Locative,
                    Inflection::Ablative,
                    Inflection::Genitive,
                ] {
                    let suffix = reading.source.source_suffix(case);
                    assert!(suffixes.insert(suffix.clone()), "{suffix}");
                    let (selected, selected_case) = definition.inflected(&suffix).unwrap();
                    assert_eq!(selected.output, reading.output);
                    assert_eq!(selected_case, case);
                }
            }
        }
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn lexical_payload_layout_is_unchanged() {
        assert_eq!(std::mem::size_of::<Lexeme>(), 64);
        assert_eq!(std::mem::size_of::<crate::SourceRange>(), 16);
        assert_eq!(std::mem::size_of::<crate::Segment>(), 64);
        assert_eq!(std::mem::size_of::<crate::Issue>(), 40);
        assert_eq!(std::mem::size_of::<crate::FallbackDiagnostic>(), 24);
        assert_eq!(std::mem::size_of::<crate::NormalizeResult>(), 136);
    }
}
