//! Bounded nominal tails, validated without allocating or reparsing spoken text.
use super::{Inflection, Spoken, Word, WordEnd};

type SuffixParts = [&'static str; 4];
const EMPTY_SUFFIX: SuffixParts = ["", "", "", ""];
const MAX_HIGH_VOWEL_BYTES: usize = "ü".len();
const MAX_PLURAL_BYTES: usize = "lar".len();
const MAX_POSSESSIVE_BYTES: usize = 2 * MAX_HIGH_VOWEL_BYTES + "mz".len();
const MAX_CASE_BYTES: usize = MAX_HIGH_VOWEL_BYTES + "nn".len();
const MAX_SUFFIX_BYTES: usize = MAX_PLURAL_BYTES + MAX_POSSESSIVE_BYTES + MAX_CASE_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Possessor {
    FirstSingular,
    SecondSingular,
    ThirdSingular,
    FirstPlural,
    SecondPlural,
    ThirdPlural,
}

const POSSESSORS: [Option<Possessor>; 7] = [
    None,
    Some(Possessor::FirstSingular),
    Some(Possessor::SecondSingular),
    Some(Possessor::ThirdSingular),
    Some(Possessor::FirstPlural),
    Some(Possessor::SecondPlural),
    Some(Possessor::ThirdPlural),
];
const CASES: [Option<Inflection>; 6] = [
    None,
    Some(Inflection::Accusative),
    Some(Inflection::Dative),
    Some(Inflection::Locative),
    Some(Inflection::Ablative),
    Some(Inflection::Genitive),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NominalInflection {
    plural: bool,
    possessor: Option<Possessor>,
    case: Option<Inflection>,
}

impl NominalInflection {
    pub(crate) fn parse(source: Word, suffix: &str) -> Option<Self> {
        let mut selected = None;
        Self::analyses(source, suffix, |form| {
            // Complete syncretic analyses are equivalent in the validated name catalog.
            selected.get_or_insert(form);
        });
        selected
    }

    pub(crate) fn analyses(source: Word, suffix: &str, mut visit: impl FnMut(Self)) {
        if suffix.is_empty() || suffix.len() > MAX_SUFFIX_BYTES {
            return;
        }
        for plural in [false, true] {
            let (stem, parts) = plural_stem(source, plural);
            let Some(remainder) = strip_parts(suffix, parts) else {
                continue;
            };
            for possessor in POSSESSORS {
                let (stem, parts, _) = possessed_stem(stem, plural, possessor);
                let Some(remainder) = strip_parts(remainder, parts) else {
                    continue;
                };
                for case in CASES {
                    if !plural && possessor.is_none() && case.is_none() {
                        continue;
                    }
                    let parts = case.map_or(EMPTY_SUFFIX, |case| stem.suffix_parts(case));
                    if strip_parts(remainder, parts) == Some("") {
                        visit(Self {
                            plural,
                            possessor,
                            case,
                        });
                    }
                }
            }
        }
    }

    pub(crate) fn forms() -> impl Iterator<Item = Self> {
        [false, true]
            .into_iter()
            .flat_map(|plural| {
                POSSESSORS.into_iter().flat_map(move |possessor| {
                    CASES.into_iter().map(move |case| Self {
                        plural,
                        possessor,
                        case,
                    })
                })
            })
            .filter(|form| form.plural || form.possessor.is_some() || form.case.is_some())
    }

    pub(crate) fn source_suffix(self, source: Word) -> String {
        let (stem, plural) = plural_stem(source, self.plural);
        let (stem, possessive, _) = possessed_stem(stem, self.plural, self.possessor);
        let case = self
            .case
            .map_or(EMPTY_SUFFIX, |case| stem.suffix_parts(case));
        plural.into_iter().chain(possessive).chain(case).collect()
    }

    pub(crate) fn render(self, output: &str, target: Word) -> String {
        let mut spoken = Spoken::lexical(output, target);
        let (stem, parts) = plural_stem(spoken.tail, self.plural);
        spoken.append_suffix(parts, false);
        spoken.tail = stem;
        let (stem, parts, vowel_start) = possessed_stem(spoken.tail, self.plural, self.possessor);
        spoken.append_suffix(parts, vowel_start);
        spoken.tail = stem;
        if let Some(case) = self.case {
            spoken.inflect(case);
        }
        spoken.into_text()
    }
}

fn strip_parts(mut source: &str, parts: SuffixParts) -> Option<&str> {
    for part in parts {
        source = source.strip_prefix(part)?;
    }
    Some(source)
}

fn plural_stem(source: Word, plural: bool) -> (Word, SuffixParts) {
    if !plural {
        return (source, EMPTY_SUFFIX);
    }
    (
        Word {
            harmony: source.harmony.flat(),
            end: WordEnd::Voiced,
            ..source
        },
        ["l", source.harmony.low(), "r", ""],
    )
}

fn possessed_stem(
    source: Word,
    plural: bool,
    possessor: Option<Possessor>,
) -> (Word, SuffixParts, bool) {
    let Some(possessor) = possessor else {
        return (source, EMPTY_SUFFIX, false);
    };
    let vowel = matches!(source.end, WordEnd::Vowel | WordEnd::Possessive);
    let high = source.harmony.high();
    let initial = if vowel { "" } else { high };
    let (harmony, end, parts, vowel_start) = match possessor {
        Possessor::FirstSingular => (
            source.harmony,
            WordEnd::Voiced,
            [initial, "m", "", ""],
            !vowel,
        ),
        Possessor::SecondSingular => (
            source.harmony,
            WordEnd::Voiced,
            [initial, "n", "", ""],
            !vowel,
        ),
        Possessor::ThirdSingular => (
            source.harmony,
            WordEnd::Possessive,
            [if vowel { "s" } else { "" }, high, "", ""],
            !vowel,
        ),
        Possessor::FirstPlural => (
            source.harmony,
            WordEnd::Voiced,
            [initial, "m", high, "z"],
            !vowel,
        ),
        Possessor::SecondPlural => (
            source.harmony,
            WordEnd::Voiced,
            [initial, "n", high, "z"],
            !vowel,
        ),
        Possessor::ThirdPlural => {
            let harmony = source.harmony.flat();
            // Noun plural and third-person plural possession share one lAr.
            let parts = if plural {
                [harmony.high(), "", "", ""]
            } else {
                ["l", source.harmony.low(), "r", harmony.high()]
            };
            (harmony, WordEnd::Possessive, parts, false)
        }
    };
    (
        Word {
            harmony,
            end,
            ..source
        },
        parts,
        vowel_start,
    )
}

#[cfg(test)]
mod tests;
