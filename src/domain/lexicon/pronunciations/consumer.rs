//! Consumer technology brands with individually approved aliases.
use super::{Lexeme, pronounced};
use crate::morphology::{
    Harmony::{BackFlat, BackRound, FrontFlat},
    WordEnd::{Voiced, Voiceless},
};

const YOUTUBE: Lexeme = pronounced("yu tub", "tub", BackRound, Voiced);
const WHATSAPP: Lexeme = pronounced("vatsap", "vatsap", BackFlat, Voiceless);
const LINKEDIN: Lexeme = pronounced("linkt in", "in", FrontFlat, Voiced);

pub(super) const ENTRIES: &[(&str, Lexeme)] = &[
    ("Google", pronounced("gugıl", "gugıl", BackFlat, Voiced)),
    ("YouTube", YOUTUBE),
    ("Youtube", YOUTUBE),
    (
        "Microsoft",
        pronounced("maykrosoft", "maykrosoft", BackRound, Voiceless),
    ),
    (
        "Windows",
        pronounced("vindovz", "vindovz", BackRound, Voiced),
    ),
    ("Apple", pronounced("epıl", "epıl", BackFlat, Voiced)),
    ("iPhone", pronounced("ayfon", "ayfon", BackRound, Voiced)),
    (
        "Samsung",
        pronounced("semsang", "semsang", BackFlat, Voiced),
    ),
    ("WhatsApp", WHATSAPP),
    ("Whatsapp", WHATSAPP),
    (
        "Instagram",
        pronounced("instagram", "instagram", BackFlat, Voiced),
    ),
    ("LinkedIn", LINKEDIN),
    ("Linkedin", LINKEDIN),
    (
        "Spotify",
        pronounced("spotıfay", "spotıfay", BackFlat, Voiced),
    ),
    ("Amazon", pronounced("emızon", "emızon", BackRound, Voiced)),
    (
        "Netflix",
        pronounced("netfliks", "netfliks", FrontFlat, Voiceless),
    ),
    ("Slack", pronounced("slek", "slek", FrontFlat, Voiceless)),
    ("Zoom", pronounced("zum", "zum", BackRound, Voiced)),
];
