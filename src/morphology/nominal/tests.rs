use super::{MAX_SUFFIX_BYTES, NominalInflection};
use crate::morphology::{Harmony, Word, WordEnd};

fn check(word: Word, cases: &[(&str, &str)]) {
    for &(suffix, expected) in cases {
        let form = NominalInflection::parse(word, suffix).unwrap();
        assert_eq!(form.render(word.text, word), expected, "{suffix}");
    }
}

#[test]
fn independent_possessive_goldens_cover_harmony_and_vowel_endings() {
    for (text, harmony, end, suffixes, outputs) in [
        (
            "ayfon",
            Harmony::BackRound,
            WordEnd::Voiced,
            ["um", "un", "u", "umuz", "unuz", "ları"],
            [
                "ayfonum",
                "ayfonun",
                "ayfonu",
                "ayfonumuz",
                "ayfonunuz",
                "ayfonları",
            ],
        ),
        (
            "gugıl",
            Harmony::BackFlat,
            WordEnd::Voiced,
            ["ım", "ın", "ı", "ımız", "ınız", "ları"],
            [
                "gugılım",
                "gugılın",
                "gugılı",
                "gugılımız",
                "gugılınız",
                "gugılları",
            ],
        ),
        (
            "kodeks",
            Harmony::FrontFlat,
            WordEnd::Voiceless,
            ["im", "in", "i", "imiz", "iniz", "leri"],
            [
                "kodeksim",
                "kodeksin",
                "kodeksi",
                "kodeksimiz",
                "kodeksiniz",
                "kodeksleri",
            ],
        ),
        (
            "göl",
            Harmony::FrontRound,
            WordEnd::Voiced,
            ["üm", "ün", "ü", "ümüz", "ünüz", "leri"],
            ["gölüm", "gölün", "gölü", "gölümüz", "gölünüz", "gölleri"],
        ),
        (
            "ti",
            Harmony::FrontFlat,
            WordEnd::Vowel,
            ["m", "n", "si", "miz", "niz", "leri"],
            ["tim", "tin", "tisi", "timiz", "tiniz", "tileri"],
        ),
        (
            "köprü",
            Harmony::FrontRound,
            WordEnd::Vowel,
            ["m", "n", "sü", "müz", "nüz", "leri"],
            [
                "köprüm",
                "köprün",
                "köprüsü",
                "köprümüz",
                "köprünüz",
                "köprüleri",
            ],
        ),
    ] {
        let word = Word::new(text, harmony, end);
        for (suffix, expected) in suffixes.into_iter().zip(outputs) {
            check(word, &[(suffix, expected)]);
        }
    }
}

#[test]
fn case_chains_use_the_current_stem_and_correct_buffers() {
    let word = Word::new("ayfon", Harmony::BackRound, WordEnd::Voiced);
    check(
        word,
        &[
            ("umu", "ayfonumu"),
            ("uma", "ayfonuma"),
            ("umda", "ayfonumda"),
            ("umdan", "ayfonumdan"),
            ("umun", "ayfonumun"),
            ("unu", "ayfonunu"),
            ("una", "ayfonuna"),
            ("unda", "ayfonunda"),
            ("undan", "ayfonundan"),
            ("unun", "ayfonunun"),
            ("umuzu", "ayfonumuzu"),
            ("umuza", "ayfonumuza"),
            ("umuzda", "ayfonumuzda"),
            ("umuzdan", "ayfonumuzdan"),
            ("umuzun", "ayfonumuzun"),
            ("unuzu", "ayfonunuzu"),
            ("unuza", "ayfonunuza"),
            ("unuzda", "ayfonunuzda"),
            ("unuzdan", "ayfonunuzdan"),
            ("unuzun", "ayfonunuzun"),
            ("larını", "ayfonlarını"),
            ("larına", "ayfonlarına"),
            ("larında", "ayfonlarında"),
            ("larından", "ayfonlarından"),
            ("larının", "ayfonlarının"),
            ("larımı", "ayfonlarımı"),
            ("larıma", "ayfonlarıma"),
            ("larımda", "ayfonlarımda"),
            ("larımdan", "ayfonlarımdan"),
            ("larımın", "ayfonlarımın"),
        ],
    );
    check(
        Word::new("göl", Harmony::FrontRound, WordEnd::Voiced),
        &[("lerimden", "göllerimden"), ("lerinin", "göllerinin")],
    );
    check(
        Word::new("ti", Harmony::FrontFlat, WordEnd::Vowel),
        &[
            ("sini", "tisini"),
            ("sine", "tisine"),
            ("sinde", "tisinde"),
            ("sinden", "tisinden"),
            ("sinin", "tisinin"),
            ("mi", "timi"),
            ("me", "time"),
            ("mde", "timde"),
            ("mden", "timden"),
            ("min", "timin"),
        ],
    );
}

#[test]
fn nominal_generation_preserves_authored_softening_and_stop_rules() {
    check(
        Word::new("kanat", Harmony::BackFlat, WordEnd::Softens),
        &[
            ("ım", "kanadım"),
            ("a", "kanada"),
            ("ta", "kanatta"),
            ("larım", "kanatlarım"),
        ],
    );
    check(
        Word::new("metreküp", Harmony::FrontRound, WordEnd::SoftensP),
        &[
            ("ümden", "metrekübümden"),
            ("ten", "metreküpten"),
            ("lerim", "metreküplerim"),
        ],
    );
}

#[test]
fn complete_syncretic_analyses_produce_identical_speech() {
    let word = Word::new("ayfon", Harmony::BackRound, WordEnd::Voiced);
    for suffix in ["u", "un", "una", "ları", "larından"] {
        let mut count = 0;
        NominalInflection::analyses(word, suffix, |form| {
            count += 1;
            assert_eq!(form.render(word.text, word), format!("ayfon{suffix}"));
        });
        assert!(count > 1, "{suffix}");
    }
}

#[test]
fn invalid_or_unbounded_tails_are_not_partially_accepted() {
    let word = Word::new("ayfon", Harmony::BackRound, WordEnd::Voiced);
    for suffix in [
        "", "üm", "ya", "nu", "umndan", "laru", "larum", "larlar", "larları", "umlar", "dandan",
        "dana", "danım", "un'dan", "umx", "umdanx", "lık",
    ] {
        assert!(NominalInflection::parse(word, suffix).is_none(), "{suffix}");
    }
    assert!(NominalInflection::parse(word, &"lar".repeat(10_000)).is_none());
}

#[test]
fn every_finite_form_round_trips_within_the_named_bound() {
    for harmony in [
        Harmony::BackFlat,
        Harmony::BackRound,
        Harmony::FrontFlat,
        Harmony::FrontRound,
    ] {
        for end in [
            WordEnd::Vowel,
            WordEnd::Voiced,
            WordEnd::Voiceless,
            WordEnd::Softens,
            WordEnd::SoftensP,
            WordEnd::Possessive,
        ] {
            let word = Word::new("test", harmony, end);
            for form in NominalInflection::forms() {
                let suffix = form.source_suffix(word);
                assert!(suffix.len() <= MAX_SUFFIX_BYTES, "{suffix}");
                let parsed = NominalInflection::parse(word, &suffix).unwrap();
                assert_eq!(parsed.render(word.text, word), form.render(word.text, word));
            }
        }
    }
}
