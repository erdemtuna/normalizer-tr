//! Countries, institutions, public services and political initialisms.
use super::{Definition, Lexeme, initialism, spelled};
use crate::morphology::{
    Harmony::{BackFlat, BackRound, FrontFlat, FrontRound},
    Word,
    WordEnd::Vowel,
};

pub(super) const ENTRIES: &[(&str, Definition)] = &[
    ("AB", initialism("a be", "be", FrontFlat)),
    ("ABD", initialism("a be de", "de", FrontFlat)),
    ("TC", initialism("te ce", "ce", FrontFlat)),
    ("BM", initialism("be me", "me", FrontFlat)),
    (
        "NATO",
        Definition::single(Lexeme::same("nato", BackRound, Vowel)),
    ),
    ("AİHM", initialism("a i he me", "me", FrontFlat)),
    (
        "TBMM",
        Definition::single(Lexeme::distinct(
            "te be me me",
            Word::new("me", FrontFlat, Vowel),
            Word::new("me", FrontFlat, Vowel),
        )),
    ),
    (
        "PTT",
        Definition::single(Lexeme::distinct(
            "pe te te",
            Word::new("te", FrontFlat, Vowel),
            Word::new("te", FrontFlat, Vowel),
        )),
    ),
    ("AYM", initialism("a ye me", "me", FrontFlat)),
    (
        "BTK",
        Definition::with_alternatives(
            spelled("be te ke", "ke", FrontFlat),
            &[spelled("be te ka", "ka", BackFlat)],
        ),
    ),
    ("DSİ", initialism("de se i", "i", FrontFlat)),
    ("DSÖ", initialism("de se ö", "ö", FrontRound)),
    ("EGM", initialism("e ge me", "me", FrontFlat)),
    ("İBB", initialism("i be be", "be", FrontFlat)),
    ("İETT", initialism("i e te te", "te", FrontFlat)),
    (
        "KVKK",
        Definition::with_alternatives(
            spelled("ke ve ke ke", "ke", FrontFlat),
            &[spelled("ke ve ke ka", "ka", BackFlat)],
        ),
    ),
    ("MHRS", initialism("me he re se", "se", FrontFlat)),
    ("MSB", initialism("me se be", "be", FrontFlat)),
    ("TCDD", initialism("te ce de de", "de", FrontFlat)),
    (
        "TCK",
        Definition::with_alternatives(
            spelled("te ce ke", "ke", FrontFlat),
            &[spelled("te ce ka", "ka", BackFlat)],
        ),
    ),
    (
        "TDK",
        Definition::with_alternatives(
            spelled("te de ke", "ke", FrontFlat),
            &[spelled("te de ka", "ka", BackFlat)],
        ),
    ),
    ("THY", initialism("te he ye", "ye", FrontFlat)),
    ("TSE", initialism("te se e", "e", FrontFlat)),
    (
        "TSK",
        Definition::with_alternatives(
            spelled("te se ke", "ke", FrontFlat),
            &[spelled("te se ka", "ka", BackFlat)],
        ),
    ),
    (
        "YSK",
        Definition::with_alternatives(
            spelled("ye se ke", "ke", FrontFlat),
            &[spelled("ye se ka", "ka", BackFlat)],
        ),
    ),
    ("CHP", initialism("ce he pe", "pe", FrontFlat)),
    ("AKP", initialism("a ke pe", "pe", FrontFlat)),
    ("BBP", initialism("be be pe", "pe", FrontFlat)),
    ("DSP", initialism("de se pe", "pe", FrontFlat)),
    ("HDP", initialism("he de pe", "pe", FrontFlat)),
    ("MHP", initialism("me he pe", "pe", FrontFlat)),
];
