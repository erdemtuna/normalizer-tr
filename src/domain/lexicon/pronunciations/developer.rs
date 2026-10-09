//! Developer tools and bounded exact product phrases.
use super::{Lexeme, pronounced};
use crate::morphology::{
    Harmony::{BackFlat, BackRound, FrontFlat},
    WordEnd::{Voiced, Voiceless},
};

const GITHUB: Lexeme = pronounced("git hab", "hab", BackFlat, Voiced);

pub(super) const ENTRIES: &[(&str, Lexeme)] = &[
    ("GitHub", GITHUB),
    ("Github", GITHUB),
    ("GITHUB", GITHUB),
    (
        "Copilot",
        pronounced("ko paylıt", "paylıt", BackFlat, Voiceless),
    ),
    ("Docker", pronounced("dokır", "dokır", BackFlat, Voiced)),
    (
        "Kubernetes",
        pronounced("kubırnetiz", "kubırnetiz", FrontFlat, Voiced),
    ),
    ("Python", pronounced("paytın", "paytın", BackFlat, Voiced)),
    (
        "JavaScript",
        pronounced("cava skript", "skript", FrontFlat, Voiceless),
    ),
    (
        "TypeScript",
        pronounced("tayp skript", "skript", FrontFlat, Voiceless),
    ),
    ("Rust", pronounced("rast", "rast", BackFlat, Voiceless)),
    ("React", pronounced("ri ekt", "ekt", FrontFlat, Voiceless)),
    (
        "GitHub Copilot",
        pronounced("git hab ko paylıt", "paylıt", BackFlat, Voiceless),
    ),
    (
        "Visual Studio Code",
        pronounced("vijuıl stüdyo kod", "kod", BackRound, Voiced),
    ),
    ("VS Code", pronounced("vi es kod", "kod", BackRound, Voiced)),
];
