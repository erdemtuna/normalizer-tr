//! Titles and prose abbreviations with explicit capitalization aliases.
use super::{Definition, Lexeme};
use crate::morphology::{
    Harmony::{BackRound, FrontFlat, FrontRound},
    Word,
    WordEnd::{Voiced, Voiceless, Vowel},
};

const DOCTOR: Definition = Definition::single(Lexeme::same("doktor", BackRound, Voiced));
const PROFESSOR: Definition = Definition::single(Lexeme::same("profesör", FrontRound, Voiced));
const DOCENT: Definition = Definition::single(Lexeme::same("doçent", FrontFlat, Voiceless));

pub(super) const ENTRIES: &[(&str, Definition)] = &[
    ("Dr.", DOCTOR),
    ("dr.", DOCTOR),
    ("DR.", DOCTOR),
    ("Prof.", PROFESSOR),
    ("prof.", PROFESSOR),
    ("PROF.", PROFESSOR),
    ("Doç.", DOCENT),
    ("doç.", DOCENT),
    ("DOÇ.", DOCENT),
    (
        "vb.",
        Definition::single(Lexeme::distinct(
            "ve benzeri",
            Word::new("be", FrontFlat, Vowel),
            Word::new("benzeri", FrontFlat, Vowel),
        )),
    ),
];
