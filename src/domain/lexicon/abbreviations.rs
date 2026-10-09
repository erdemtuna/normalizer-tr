//! Approved lexical readings, separate from source traversal and policy handling.
use super::Lexeme;
use crate::morphology::{
    Harmony,
    Harmony::{BackFlat, BackRound, FrontFlat, FrontRound},
    Inflection, Word,
    WordEnd::{Possessive, Voiced, Voiceless, Vowel},
    case_inflection,
};

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

const DOCTOR: Definition = Definition::single(Lexeme::same("doktor", BackRound, Voiced));

const PROFESSOR: Definition = Definition::single(Lexeme::same("profesör", FrontRound, Voiced));

const DOCENT: Definition = Definition::single(Lexeme::same("doçent", FrontFlat, Voiceless));

const ENTRIES: &[(&str, Definition)] = &[
    ("AB", initialism("a be", "be", FrontFlat)),
    ("ABD", initialism("a be de", "de", FrontFlat)),
    ("AKP", initialism("a ke pe", "pe", FrontFlat)),
    ("API", initialism("a pe i", "i", FrontFlat)),
    ("AYM", initialism("a ye me", "me", FrontFlat)),
    ("AİHM", initialism("a i he me", "me", FrontFlat)),
    ("BBP", initialism("be be pe", "pe", FrontFlat)),
    (
        "BDDK",
        Definition::with_alternatives(
            spelled("be de de ke", "ke", FrontFlat),
            &[spelled("be de de ka", "ka", BackFlat)],
        ),
    ),
    ("BM", initialism("be me", "me", FrontFlat)),
    ("BSMV", initialism("be se me ve", "ve", FrontFlat)),
    (
        "BTK",
        Definition::with_alternatives(
            spelled("be te ke", "ke", FrontFlat),
            &[spelled("be te ka", "ka", BackFlat)],
        ),
    ),
    ("CHP", initialism("ce he pe", "pe", FrontFlat)),
    ("CPU", initialism("ce pe u", "u", BackRound)),
    ("CSS", initialism("ce se se", "se", FrontFlat)),
    ("DOÇ.", DOCENT),
    ("DR.", DOCTOR),
    ("DSP", initialism("de se pe", "pe", FrontFlat)),
    ("DSÖ", initialism("de se ö", "ö", FrontRound)),
    ("DSİ", initialism("de se i", "i", FrontFlat)),
    ("DVD", initialism("de ve de", "de", FrontFlat)),
    ("Doç.", DOCENT),
    ("Dr.", DOCTOR),
    ("EFT", initialism("e fe te", "te", FrontFlat)),
    ("EGM", initialism("e ge me", "me", FrontFlat)),
    ("GPS", initialism("ge pe se", "se", FrontFlat)),
    ("GPU", initialism("ge pe u", "u", BackRound)),
    ("GSM", initialism("ge se me", "me", FrontFlat)),
    ("HDMI", initialism("he de me i", "i", FrontFlat)),
    ("HDP", initialism("he de pe", "pe", FrontFlat)),
    ("HTML", initialism("he te me le", "le", FrontFlat)),
    ("HTTP", initialism("he te te pe", "pe", FrontFlat)),
    ("HTTPS", initialism("he te te pe se", "se", FrontFlat)),
    (
        "IBAN",
        Definition::single(Lexeme::same("iban", BackFlat, Voiced)),
    ),
    ("IP", initialism("i pe", "pe", FrontFlat)),
    (
        "KDV",
        Definition::single(Lexeme::distinct(
            "katma değer vergisi",
            Word::new("ve", FrontFlat, Vowel),
            Word::new("vergisi", FrontFlat, Possessive),
        )),
    ),
    ("KPSS", initialism("ke pe se se", "se", FrontFlat)),
    ("KTÜ", initialism("ke te ü", "ü", FrontRound)),
    (
        "KVKK",
        Definition::with_alternatives(
            spelled("ke ve ke ke", "ke", FrontFlat),
            &[spelled("ke ve ke ka", "ka", BackFlat)],
        ),
    ),
    ("LCD", initialism("le ce de", "de", FrontFlat)),
    ("LGS", initialism("le ge se", "se", FrontFlat)),
    ("MHP", initialism("me he pe", "pe", FrontFlat)),
    ("MHRS", initialism("me he re se", "se", FrontFlat)),
    ("MSB", initialism("me se be", "be", FrontFlat)),
    (
        "NATO",
        Definition::single(Lexeme::same("nato", BackRound, Vowel)),
    ),
    (
        "PDF",
        Definition::with_alternatives(
            spelled("pe de fe", "fe", FrontFlat),
            &[Lexeme::distinct(
                "pe de ef",
                Word::new("ef", FrontFlat, Voiceless),
                Word::new("ef", FrontFlat, Voiceless),
            )],
        ),
    ),
    ("PROF.", PROFESSOR),
    (
        "PTT",
        Definition::single(Lexeme::distinct(
            "pe te te",
            Word::new("te", FrontFlat, Vowel),
            Word::new("te", FrontFlat, Vowel),
        )),
    ),
    ("Prof.", PROFESSOR),
    (
        "SGK",
        Definition::with_alternatives(
            spelled("se ge ka", "ka", BackFlat),
            &[spelled("se ge ke", "ke", FrontFlat)],
        ),
    ),
    ("SMS", initialism("se me se", "se", FrontFlat)),
    (
        "SPK",
        Definition::with_alternatives(
            spelled("se pe ke", "ke", FrontFlat),
            &[spelled("se pe ka", "ka", BackFlat)],
        ),
    ),
    (
        "SSK",
        Definition::with_alternatives(
            spelled("se se ke", "ke", FrontFlat),
            &[spelled("se se ka", "ka", BackFlat)],
        ),
    ),
    (
        "TBMM",
        Definition::single(Lexeme::distinct(
            "te be me me",
            Word::new("me", FrontFlat, Vowel),
            Word::new("me", FrontFlat, Vowel),
        )),
    ),
    ("TC", initialism("te ce", "ce", FrontFlat)),
    ("TCDD", initialism("te ce de de", "de", FrontFlat)),
    (
        "TCK",
        Definition::with_alternatives(
            spelled("te ce ke", "ke", FrontFlat),
            &[spelled("te ce ka", "ka", BackFlat)],
        ),
    ),
    ("TCMB", initialism("te ce me be", "be", FrontFlat)),
    (
        "TDK",
        Definition::with_alternatives(
            spelled("te de ke", "ke", FrontFlat),
            &[spelled("te de ka", "ka", BackFlat)],
        ),
    ),
    ("THY", initialism("te he ye", "ye", FrontFlat)),
    ("TMSF", initialism("te me se fe", "fe", FrontFlat)),
    ("TRT", initialism("te re te", "te", FrontFlat)),
    ("TSE", initialism("te se e", "e", FrontFlat)),
    (
        "TSK",
        Definition::with_alternatives(
            spelled("te se ke", "ke", FrontFlat),
            &[spelled("te se ka", "ka", BackFlat)],
        ),
    ),
    ("TV", initialism("te ve", "ve", FrontFlat)),
    ("URL", initialism("u re le", "le", FrontFlat)),
    ("USB", initialism("u se be", "be", FrontFlat)),
    ("XML", initialism("iks me le", "le", FrontFlat)),
    ("YDS", initialism("ye de se", "se", FrontFlat)),
    ("YKS", initialism("ye ke se", "se", FrontFlat)),
    (
        "YSK",
        Definition::with_alternatives(
            spelled("ye se ke", "ke", FrontFlat),
            &[spelled("ye se ka", "ka", BackFlat)],
        ),
    ),
    ("YTÜ", initialism("ye te ü", "ü", FrontRound)),
    ("doç.", DOCENT),
    ("dr.", DOCTOR),
    ("prof.", PROFESSOR),
    (
        "vb.",
        Definition::single(Lexeme::distinct(
            "ve benzeri",
            Word::new("be", FrontFlat, Vowel),
            Word::new("benzeri", FrontFlat, Vowel),
        )),
    ),
    ("ÖSYM", initialism("ö se ye me", "me", FrontFlat)),
    ("ÖTV", initialism("ö te ve", "ve", FrontFlat)),
    ("İBB", initialism("i be be", "be", FrontFlat)),
    ("İETT", initialism("i e te te", "te", FrontFlat)),
    ("İTÜ", initialism("i te ü", "ü", FrontRound)),
];

const MAX_KEY_BYTES: usize = {
    let mut maximum = 0;
    let mut index = 0;
    while index < ENTRIES.len() {
        if ENTRIES[index].0.len() > maximum {
            maximum = ENTRIES[index].0.len();
        }
        index += 1;
    }
    maximum
};

pub(super) fn lookup(symbol: &str) -> Option<&'static Definition> {
    if symbol.len() > MAX_KEY_BYTES
        || (!symbol.ends_with('.') && !symbol.chars().all(char::is_uppercase))
    {
        return None;
    }
    ENTRIES
        .binary_search_by(|(key, _)| key.cmp(&symbol))
        .ok()
        .map(|index| &ENTRIES[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_keys_are_unique_and_sorted() {
        assert_eq!(ENTRIES.len(), 80);
        assert!(ENTRIES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (key, definition) in ENTRIES {
            assert!(key.ends_with('.') || key.chars().all(char::is_uppercase));
            assert_eq!(
                lookup(key).unwrap().default.output,
                definition.default.output
            );
        }
    }

    #[test]
    fn variant_suffixes_are_unambiguous() {
        for symbol in [
            "SGK", "BDDK", "BTK", "KVKK", "SPK", "TCK", "TDK", "TSK", "YSK", "SSK", "PDF",
        ] {
            let definition = lookup(symbol).unwrap();
            let mut suffixes = BTreeSet::new();
            for reading in std::iter::once(&definition.default).chain(definition.alternatives) {
                assert_eq!(reading.output.rsplit(' ').next(), Some(reading.target.text));
                for case in [
                    Inflection::Accusative,
                    Inflection::Dative,
                    Inflection::Locative,
                    Inflection::Ablative,
                    Inflection::Genitive,
                ] {
                    let suffix = reading.source.source_suffix(case);
                    assert!(suffixes.insert(suffix.clone()), "{symbol} {suffix}");
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
