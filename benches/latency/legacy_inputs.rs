//! Original long-input recipes, prepared independently of clocks and normalization.
use normalizer_tr::{AmbiguityPolicy, Hint, HintKind, NormalizeOptions, SourceRange};

use super::support::ScalingCase;

pub(super) fn expanded() -> Vec<ScalingCase> {
    let mut cases = Vec::new();
    for size in [4096, 16384, 32768] {
        for (id, pattern) in [
            ("ungrouped-run", "123 "),
            ("grouped-money", "1 234,50TL; "),
            ("quoted-list", "‘25TL’,5kg; "),
            ("unmatched-quotes", "'25TL 123 "),
            ("invalid-group", "12 34,50 TL; "),
            ("decomposed", "o\u{308} 5kg; "),
            ("pronunciation-name", "Claude; "),
            ("pronunciation-phrase", "GitHub Copilot; "),
            ("pronunciation-prefix-miss", "Visual Studio Nope; "),
            ("pronunciation-list", "ChatGPT,Claude; "),
            ("pronunciation-possessive", "iPhone'umdan; "),
            ("pronunciation-plural-possessive", "iPhone'larımıza; "),
            ("pronunciation-syncretic", "iPhone'una; "),
            ("pronunciation-nominal-phrase", "GitHub Copilot'ımızda; "),
            ("pronunciation-invalid-harmony", "Instagram'de; "),
            ("pronunciation-long-tail", "lar"),
            ("malformed-comma-token", "1,"),
            ("malformed-dot-token", "1."),
            ("mixed-quantity-list", "25kg;CHP,AKP; "),
            ("rejected-phrase-list", "Claude,Hugging Face,UNKNOWN; "),
            ("number-tail", "2026'dakidir; "),
            ("quantity-possession", "25,50 TL'm; "),
            ("rate-tail", "5 km/sa'tir; "),
            ("clock-range", "13:00-17:00 arasında; "),
            ("date-source-tail", "tarih 2026-12-31'de; "),
            ("account-period", "TR33 0006 1005 1978 6457 8413 26.; "),
            (
                "account-malformed-continuation",
                "TR33 0006 1005 1978 6457 8413 26 00abc; ",
            ),
            ("address-fields", "Kat:3 Daire:7; adres: "),
            ("address-invalid-field", "Kat:xyz; adres: "),
            ("multiplication-chain", "*1"),
            ("multiplication-invalid-chain", "1**"),
        ] {
            let prefix = match id {
                "pronunciation-long-tail" => "iPhone'",
                "address-fields" | "address-invalid-field" => "adres: ",
                "multiplication-chain" => "1",
                _ => "",
            };
            let remaining = size - prefix.len();
            let mut text = String::with_capacity(size);
            text.push_str(prefix);
            text.push_str(&pattern.repeat(remaining / pattern.len()));
            text.push_str(&" ".repeat(size - text.len()));
            for ambiguity_policy in [
                AmbiguityPolicy::Preserve,
                AmbiguityPolicy::Reject,
                AmbiguityPolicy::Fallback,
            ] {
                cases.push(ScalingCase {
                    id,
                    text: text.clone(),
                    options: NormalizeOptions {
                        ambiguity_policy,
                        ..Default::default()
                    },
                });
            }
        }
    }
    cases
}

pub(super) fn large() -> Vec<ScalingCase> {
    let mut cases = Vec::new();
    for size in [4096, 16384, 32768] {
        for (id, pattern) in [
            ("verbatim", "sözcük "),
            ("candidates", "1 "),
            ("amplification", "999999999999999999 "),
        ] {
            let mut text = pattern.repeat(size / pattern.len());
            text.push_str(&" ".repeat(size - text.len()));
            cases.push(ScalingCase {
                id,
                text,
                options: NormalizeOptions::default(),
            });
        }
    }
    cases
}

pub(super) fn limits() -> Vec<ScalingCase> {
    [
        (
            "maximum-hints",
            "1;".repeat(256),
            NormalizeOptions {
                hints: (0..256)
                    .map(|i| Hint::new(SourceRange::new(i * 2, i * 2 + 1), HintKind::Cardinal))
                    .collect(),
                ..Default::default()
            },
        ),
        (
            "candidate-limit",
            "1 ".repeat(4097),
            NormalizeOptions::default(),
        ),
        (
            "long-identifier",
            format!("https://ornek.com/{}", "a".repeat(16000)),
            NormalizeOptions::default(),
        ),
        (
            "invalid-compounds",
            "TR33 0006 1005 1978 6457 8413 2A; ".repeat(400),
            NormalizeOptions::default(),
        ),
    ]
    .into_iter()
    .map(|(id, text, options)| ScalingCase { id, text, options })
    .collect()
}
