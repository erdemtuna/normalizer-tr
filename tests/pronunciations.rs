//! Independent spoken-name, phrase ownership and pronunciation provenance contracts.
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackStrategy, Hint, HintKind, IssueCategory, LimitKind,
    NormalizeError, NormalizeOptions, Normalizer, SegmentKind, SourceRange, WorkControl,
};

const READINGS: &[(&str, &str, [&str; 5])] = &[
    ("Claude", "klod", ["u", "a", "da", "dan", "un"]),
    ("Codex", "kodeks", ["i", "e", "te", "ten", "in"]),
    ("ChatGPT", "çet ci pi ti", ["yi", "ye", "de", "den", "nin"]),
    ("OpenAI", "opın ey ay", ["ı", "a", "da", "dan", "ın"]),
    ("Gemini", "ceminay", ["ı", "a", "da", "dan", "ın"]),
    ("Anthropic", "entropik", ["i", "e", "te", "ten", "in"]),
    ("GitHub", "git hab", ["ı", "a", "da", "dan", "ın"]),
    ("Copilot", "ko paylıt", ["ı", "a", "ta", "tan", "ın"]),
    ("Docker", "dokır", ["ı", "a", "da", "dan", "ın"]),
    ("Kubernetes", "kubırnetiz", ["i", "e", "de", "den", "in"]),
    ("Python", "paytın", ["ı", "a", "da", "dan", "ın"]),
    ("JavaScript", "cava skript", ["i", "e", "te", "ten", "in"]),
    ("TypeScript", "tayp skript", ["i", "e", "te", "ten", "in"]),
    ("Rust", "rast", ["ı", "a", "ta", "tan", "ın"]),
    ("React", "ri ekt", ["i", "e", "te", "ten", "in"]),
    ("Google", "gugıl", ["ı", "a", "da", "dan", "ın"]),
    ("YouTube", "yu tub", ["u", "a", "da", "dan", "un"]),
    ("Microsoft", "maykrosoft", ["u", "a", "ta", "tan", "un"]),
    ("Windows", "vindovz", ["u", "a", "da", "dan", "un"]),
    ("Apple", "epıl", ["ı", "a", "da", "dan", "ın"]),
    ("iPhone", "ayfon", ["u", "a", "da", "dan", "un"]),
    ("Samsung", "semsang", ["ı", "a", "da", "dan", "ın"]),
    ("WhatsApp", "vats ep", ["i", "e", "te", "ten", "in"]),
    ("Instagram", "instıgrem", ["i", "e", "de", "den", "in"]),
    ("LinkedIn", "linkt in", ["i", "e", "de", "den", "in"]),
    ("Spotify", "spotıfay", ["ı", "a", "da", "dan", "ın"]),
    ("Amazon", "emızon", ["u", "a", "da", "dan", "un"]),
    ("Netflix", "netfliks", ["i", "e", "te", "ten", "in"]),
    ("Slack", "slek", ["i", "e", "te", "ten", "in"]),
    ("Zoom", "zum", ["u", "a", "da", "dan", "un"]),
    (
        "GitHub Copilot",
        "git hab ko paylıt",
        ["ı", "a", "ta", "tan", "ın"],
    ),
    ("Hugging Face", "haging feys", ["i", "e", "te", "ten", "in"]),
    (
        "Visual Studio Code",
        "vijuıl stüdyo kod",
        ["u", "a", "da", "dan", "un"],
    ),
    ("VS Code", "vi es kod", ["u", "a", "da", "dan", "un"]),
];

const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

#[test]
fn new_provenance_variants_do_not_reorder_existing_enum_tags() {
    assert_eq!(SegmentKind::Range as usize, 12);
    assert_eq!(SegmentKind::Fallback as usize, 18);
    assert_eq!(SegmentKind::Pronunciation as usize, 19);
    assert_eq!(FallbackClass::Identifier as usize, 6);
    assert_eq!(FallbackClass::Symbol as usize, 10);
    assert_eq!(FallbackClass::Pronunciation as usize, 11);
}

fn options(policy: AmbiguityPolicy) -> NormalizeOptions {
    NormalizeOptions {
        ambiguity_policy: policy,
        ..Default::default()
    }
}

fn primary(n: &Normalizer, source: &str, expected: &str) {
    for policy in POLICIES {
        let result = n.normalize(source, &options(policy)).unwrap();
        assert_eq!(result.normalized_text(), expected, "{source} {policy:?}");
        assert!(result.complete() && result.issues().is_empty() && result.fallbacks().is_empty());
        assert_eq!(result.segments().len(), 1, "{source}");
        let segment = &result.segments()[0];
        assert_eq!(segment.kind(), SegmentKind::Pronunciation);
        assert_eq!(segment.rule_id(), "pronunciation.name");
        assert_eq!(segment.range(), SourceRange::new(0, source.len()));
    }
}

#[test]
fn approved_defaults_and_five_case_families_have_independent_goldens() {
    let n = Normalizer::new().unwrap();
    assert_eq!(READINGS.len(), 34);
    for &(key, reading, suffixes) in READINGS {
        primary(&n, key, reading);
        for suffix in suffixes {
            primary(
                &n,
                &format!("{key}'{suffix}"),
                &format!("{reading}{suffix}"),
            );
            primary(
                &n,
                &format!("{key}’{suffix}"),
                &format!("{reading}{suffix}"),
            );
        }
    }
}

#[test]
fn only_explicit_casing_aliases_are_accepted() {
    let n = Normalizer::new().unwrap();
    for (key, expected) in [
        ("CLAUDE", "klod"),
        ("CODEX", "kodeks"),
        ("CHATGPT", "çet ci pi ti"),
        ("chatgpt", "çet ci pi ti"),
        ("OPENAI", "opın ey ay"),
        ("Github", "git hab"),
        ("GITHUB", "git hab"),
        ("Youtube", "yu tub"),
        ("Whatsapp", "vats ep"),
        ("Linkedin", "linkt in"),
    ] {
        primary(&n, key, expected);
    }
    for source in [
        "apple react codex rust python",
        "claude",
        "gemini",
        "GitHUB",
        "GEMINI",
        "Face Studio Code",
        "Visual",
        "Hugging",
        "UnknownProduct",
        "MyChatGPTTool",
        "ClaudeMonet",
        "CodexTool",
    ] {
        for policy in POLICIES {
            let result = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(result.normalized_text(), source);
            assert!(result.complete() && result.fallbacks().is_empty());
            assert!(
                result
                    .segments()
                    .iter()
                    .all(|segment| segment.kind() == SegmentKind::Verbatim)
            );
        }
    }
}

#[test]
fn phrases_have_longest_exact_claims_and_do_not_cross_other_whitespace() {
    let n = Normalizer::new().unwrap();
    for source in [
        "GitHub Copilot",
        "Visual Studio Code",
        "Hugging Face",
        "VS Code",
    ] {
        let result = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(result.segments().len(), 1);
        assert_eq!(
            result.segments()[0].range(),
            SourceRange::new(0, source.len())
        );
    }
    for source in ["GitHub\tCopilot", "GitHub\nCopilot", "GitHub  Copilot"] {
        let result = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(
            result
                .segments()
                .iter()
                .filter(|segment| segment.kind() == SegmentKind::Pronunciation)
                .count(),
            2
        );
        assert!(result.normalized_text().starts_with("git hab"));
        assert!(result.normalized_text().ends_with("ko paylıt"));
    }
    for source in [
        "Visual\tStudio Code",
        "Hugging\nFace",
        "Visual  Studio Code",
    ] {
        let result = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(result.normalized_text(), source);
    }
}

#[test]
fn punctuation_and_original_unicode_coordinates_stay_source_faithful() {
    let n = Normalizer::new().unwrap();
    for (source, expected) in [
        ("ChatGPT,Claude", "çet ci pi ti,klod"),
        ("Hugging Face,Claude", "haging feys,klod"),
        ("Claude,Hugging Face", "klod,haging feys"),
        ("Visual Studio Code,Claude", "vijuıl stüdyo kod,klod"),
        ("Claude,Visual Studio Code.", "klod,vijuıl stüdyo kod."),
        ("Face,Claude", "Face,Claude"),
        ("Code,Claude", "Code,Claude"),
        ("Claude'a UNKNOWN,GitHub", "kloda UNKNOWN,GitHub"),
        ("Claude'a,ChatGPT'ye", "kloda,çet ci pi tiye"),
        ("‘Claude’un’", "‘klodun’"),
        ("Claude:", "klod:"),
        ("Visual Studio Code:", "vijuıl stüdyo kod:"),
        ("GitHub Copilot'ın.", "git hab ko paylıtın."),
        ("ö Claude’un!", "ö klodun!"),
        ("o\u{308} Claude’un!", "o\u{308} klodun!"),
        ("Claude;Codex", "klod;kodeks"),
        ("Claude,UNKNOWN", "Claude,UNKNOWN"),
    ] {
        for policy in POLICIES {
            let result = n.normalize(source, &options(policy)).unwrap();
            assert_eq!(result.normalized_text(), expected, "{source}");
            assert!(result.complete() && !result.fallback_used());
            let mut cursor = 0;
            for segment in result.segments() {
                assert_eq!(segment.range().start(), cursor);
                cursor = segment.range().end();
                assert!(source.is_char_boundary(cursor));
                if segment.kind() == SegmentKind::Verbatim {
                    assert_eq!(segment.text(), &source[segment.range().start()..cursor]);
                }
            }
            assert_eq!(cursor, source.len());
            assert_eq!(
                result
                    .segments()
                    .iter()
                    .map(|segment| segment.text())
                    .collect::<String>(),
                expected
            );
        }
    }
}

#[test]
fn failed_suffixes_keep_whole_name_findings_without_repair() {
    let n = Normalizer::new().unwrap();
    for (source, category) in [
        ("Claude'ye", IssueCategory::InvalidExpression),
        ("Claude'un'dan", IssueCategory::Unsupported),
        ("ChatGPT'", IssueCategory::Unsupported),
        ("Visual Studio Code'ye", IssueCategory::InvalidExpression),
    ] {
        let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
        assert_eq!(preserve.normalized_text(), source);
        assert_eq!(preserve.issues().len(), 1);
        assert_eq!(preserve.issues()[0].category(), category);
        assert_eq!(
            preserve.issues()[0].range(),
            SourceRange::new(0, source.len())
        );
        assert_eq!(
            n.normalize(source, &options(AmbiguityPolicy::Reject)),
            Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
        );
        let fallback = n
            .normalize(source, &options(AmbiguityPolicy::Fallback))
            .unwrap();
        assert!(fallback.complete() && fallback.fallback_used());
        assert_eq!(fallback.fallbacks().len(), 1);
        let record = &fallback.fallbacks()[0];
        assert_eq!(record.attempted_class(), FallbackClass::Pronunciation);
        assert_eq!(record.strategy(), FallbackStrategy::Literal);
        assert_eq!(record.original_category(), Some(category));
        assert_eq!(record.range(), SourceRange::new(0, source.len()));
    }
}

#[test]
fn failed_names_and_other_unresolved_work_are_collected_before_rejection() {
    let n = Normalizer::new().unwrap();
    let source = "Claude'ye; 1.234; Codex'ya";
    let preserve = n.normalize(source, &NormalizeOptions::default()).unwrap();
    assert_eq!(preserve.issues().len(), 3);
    assert_eq!(
        preserve
            .issues()
            .iter()
            .map(|issue| issue.range())
            .collect::<Vec<_>>(),
        [
            SourceRange::new(0, 9),
            SourceRange::new(11, 16),
            SourceRange::new(18, 26)
        ]
    );
    assert_eq!(
        n.normalize(source, &options(AmbiguityPolicy::Reject)),
        Err(NormalizeError::Unresolved(preserve.issues().to_vec()))
    );
    let fallback = n
        .normalize(source, &options(AmbiguityPolicy::Fallback))
        .unwrap();
    assert!(fallback.complete() && fallback.issues().is_empty());
    assert_eq!(fallback.fallbacks().len(), 3);
}

#[test]
fn electronic_identifiers_versions_and_explicit_hints_keep_ownership() {
    let n = Normalizer::new().unwrap();
    for source in [
        "Claude.ai",
        "claude@example.com",
        "https://ornek.com/Claude?x=ChatGPT",
        "GPT-5.4",
        "ChatGPT4o",
        "Claude-3.5",
        "AB12(Claude)",
        "Apple123",
        "Visual Studio Code'foo@example.com",
        "Hugging Face@example.com",
        "Claude,Hugging Face'foo@example.com",
    ] {
        for policy in POLICIES {
            match n.normalize(source, &options(policy)) {
                Ok(result) => assert!(
                    result
                        .segments()
                        .iter()
                        .all(|segment| segment.kind() != SegmentKind::Pronunciation),
                    "{source}"
                ),
                Err(error) => assert!(matches!(error, NormalizeError::Unresolved(_))),
            }
        }
    }
    for source in ["Claude", "GitHub Copilot"] {
        for policy in POLICIES {
            let mut options = options(policy);
            options
                .hints
                .push(Hint::new(SourceRange::new(0, 2), HintKind::Cardinal));
            assert_eq!(
                n.normalize(source, &options),
                Err(NormalizeError::InvalidHint)
            );
        }
    }
}

#[test]
fn new_primary_readings_respect_engineering_limits_and_controls() {
    let n = Normalizer::new().unwrap();
    let control = WorkControl::default();
    control.cancel();
    assert_eq!(
        n.normalize_controlled("Claude", &NormalizeOptions::default(), &control),
        Err(NormalizeError::Cancelled)
    );
    let source = vec!["Claude"; normalizer_tr::MAX_CANDIDATES + 1].join(" ");
    for policy in POLICIES {
        assert_eq!(
            n.normalize(&source, &options(policy)),
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates))
        );
        assert_eq!(
            n.normalize("Claude\u{202e}", &options(policy)),
            Err(NormalizeError::InvalidInput)
        );
    }
    let amplified = format!("Claude'{}", "🫠".repeat(8000));
    assert!(amplified.len() <= normalizer_tr::MAX_INPUT_BYTES);
    assert_eq!(
        n.normalize(&amplified, &options(AmbiguityPolicy::Fallback)),
        Err(NormalizeError::LimitExceeded(LimitKind::Result))
    );
}

#[cfg(feature = "serde")]
#[test]
fn pronunciation_labels_are_serialized_as_distinct_semantic_values() {
    let n = Normalizer::new().unwrap();
    let primary = n.normalize("Claude", &NormalizeOptions::default()).unwrap();
    let serialized = serde_json::to_value(primary).unwrap();
    assert_eq!(serialized["segments"][0]["kind"], "Pronunciation");
    assert_eq!(serialized["segments"][0]["rule_id"], "pronunciation.name");
    let result = n
        .normalize("Claude'ye", &options(AmbiguityPolicy::Fallback))
        .unwrap();
    let serialized = serde_json::to_value(result).unwrap();
    assert_eq!(
        serialized["fallbacks"][0]["attempted_class"],
        "Pronunciation"
    );
}
