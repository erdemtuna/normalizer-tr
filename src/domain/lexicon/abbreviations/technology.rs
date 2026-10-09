//! Computing, communications, media and electronic-format initialisms.
use super::{Definition, Lexeme, initialism, spelled};
use crate::morphology::{
    Harmony::{BackRound, FrontFlat},
    Word,
    WordEnd::Voiceless,
};

pub(super) const ENTRIES: &[(&str, Definition)] = &[
    ("CPU", initialism("ce pe u", "u", BackRound)),
    ("GPU", initialism("ge pe u", "u", BackRound)),
    ("API", initialism("a pe i", "i", FrontFlat)),
    ("IP", initialism("i pe", "pe", FrontFlat)),
    ("HDMI", initialism("he de me i", "i", FrontFlat)),
    ("USB", initialism("u se be", "be", FrontFlat)),
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
    ("URL", initialism("u re le", "le", FrontFlat)),
    ("HTTP", initialism("he te te pe", "pe", FrontFlat)),
    ("HTTPS", initialism("he te te pe se", "se", FrontFlat)),
    ("HTML", initialism("he te me le", "le", FrontFlat)),
    ("XML", initialism("iks me le", "le", FrontFlat)),
    ("CSS", initialism("ce se se", "se", FrontFlat)),
    ("SMS", initialism("se me se", "se", FrontFlat)),
    ("GPS", initialism("ge pe se", "se", FrontFlat)),
    ("GSM", initialism("ge se me", "me", FrontFlat)),
    ("DVD", initialism("de ve de", "de", FrontFlat)),
    ("LCD", initialism("le ce de", "de", FrontFlat)),
    ("TV", initialism("te ve", "ve", FrontFlat)),
    ("TRT", initialism("te re te", "te", FrontFlat)),
];
