//! AI products and exact provider names.
use super::{Lexeme, pronounced};
use crate::morphology::{
    Harmony::{BackFlat, BackRound, FrontFlat},
    WordEnd::{Voiced, Voiceless, Vowel},
};

const CLAUDE: Lexeme = pronounced("klod", "klod", BackRound, Voiced);
const CODEX: Lexeme = pronounced("kodeks", "kodeks", FrontFlat, Voiceless);
const CHATGPT: Lexeme = pronounced("çet ci pi ti", "ti", FrontFlat, Vowel);
const OPENAI: Lexeme = pronounced("opın ey ay", "ay", BackFlat, Voiced);
const EMA_LIGHTNING: Lexeme = pronounced("ema laytning", "laytning", FrontFlat, Voiced);

pub(super) const ENTRIES: &[(&str, Lexeme)] = &[
    ("Claude", CLAUDE),
    ("CLAUDE", CLAUDE),
    ("Codex", CODEX),
    ("CODEX", CODEX),
    ("ChatGPT", CHATGPT),
    ("CHATGPT", CHATGPT),
    ("chatgpt", CHATGPT),
    ("OpenAI", OPENAI),
    ("OPENAI", OPENAI),
    ("Gemini", pronounced("ceminay", "ceminay", BackFlat, Voiced)),
    (
        "Anthropic",
        pronounced("entropik", "entropik", FrontFlat, Voiceless),
    ),
    (
        "Hugging Face",
        pronounced("haging feys", "feys", FrontFlat, Voiceless),
    ),
    ("EMA Lightning", EMA_LIGHTNING),
    ("ema lightning", EMA_LIGHTNING),
    ("ema-lightning", EMA_LIGHTNING),
];
