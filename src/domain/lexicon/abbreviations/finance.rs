//! Finance, taxation, banking and social insurance abbreviations.
use super::{Definition, Lexeme, initialism, spelled};
use crate::morphology::{
    Harmony::{BackFlat, FrontFlat},
    Word,
    WordEnd::{Possessive, Voiced, Vowel},
};

pub(super) const ENTRIES: &[(&str, Definition)] = &[
    (
        "IBAN",
        Definition::single(Lexeme::same("iban", BackFlat, Voiced)),
    ),
    (
        "KDV",
        Definition::single(Lexeme::distinct(
            "katma değer vergisi",
            Word::new("ve", FrontFlat, Vowel),
            Word::new("vergisi", FrontFlat, Possessive),
        )),
    ),
    (
        "SGK",
        Definition::with_alternatives(
            spelled("se ge ka", "ka", BackFlat),
            &[spelled("se ge ke", "ke", FrontFlat)],
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
        "BDDK",
        Definition::with_alternatives(
            spelled("be de de ke", "ke", FrontFlat),
            &[spelled("be de de ka", "ka", BackFlat)],
        ),
    ),
    ("BSMV", initialism("be se me ve", "ve", FrontFlat)),
    ("EFT", initialism("e fe te", "te", FrontFlat)),
    ("ÖTV", initialism("ö te ve", "ve", FrontFlat)),
    (
        "SPK",
        Definition::with_alternatives(
            spelled("se pe ke", "ke", FrontFlat),
            &[spelled("se pe ka", "ka", BackFlat)],
        ),
    ),
    ("TCMB", initialism("te ce me be", "be", FrontFlat)),
    ("TMSF", initialism("te me se fe", "fe", FrontFlat)),
];
