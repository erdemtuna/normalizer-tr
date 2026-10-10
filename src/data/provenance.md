# Data provenance

One authored normalizer, one diagnostic package/build identity. Typed lexical/
phonological inventory is in [domain/lexicon.rs](../domain/lexicon.rs); patterns
are validated/cached once in [resources.rs](../resources.rs). There is no
selectable profile, older table or
older-version mode. Preserve/Reject/Fallback are per-call resolution policies.

Fallback data and source rendering incorporate parts of Canberk Aslan's
Apache-2.0 contribution: https://github.com/erdemtuna/normalizer-tr/pull/1,
head `0a5de68286cacc4f95c754f519899d15fc2c5e73`.
The inventory is in [notation/symbols.rs](../notation/symbols.rs);
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) retains attribution.

The [initialism catalog](../domain/lexicon/abbreviations.rs) was reviewed against
that contribution, with a bounded set of exact keys and explicitly approved
pronunciation variants. Suffixes select typed readings; the catalog is not an
acronym detector or a copy of the contribution's contextual abbreviation engine.

The [foreign-name catalog](../domain/lexicon/pronunciations.rs) contains finite,
user-reviewed text mappings authored in thematic AI/developer/consumer groups.
Explicit spelling aliases and Turkish suffix tails accompany each reading.
These Turkish-readable aliases are not IPA, an official pronunciation dictionary
or a claim of universal/native TTS pronunciation. No speech/audio model was
tested, and no bulk dictionary or external model dataset is bundled.
See [the exact coverage](../../docs/normalization.md#approved-foreign-names).

The nominal suffix rules and regression examples are authored in this project.
No external morphology library, dictionary or model data is added.

Owned Rust code, unit/currency/abbreviation/name/character readings and finite
source/target/derived-tail metadata are Apache-2.0. No bulk dictionary or
third-party normalizer source was copied. Exact numeral behavior was
cross-checked against Unicode CLDR release 48 Turkish RBNF:
https://github.com/unicode-org/cldr/blob/release-48/common/rbnf/tr.xml
(Copyright 1991-2025 Unicode; Unicode License V3).
The notice is retained in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md);
CLDR is a reference, not a bundled runtime grammar or whole-sentence oracle.

TDK number/apostrophe/abbreviation writing guidance informed the bounded rules:
https://tdk.gov.tr/icerik/yazim-kurallari/sayilarin-yazilisi/
https://tdk.gov.tr/icerik/yazim-kurallari/kesme-isareti/
https://tdk.gov.tr/icerik/yazim-kurallari/kisaltmalar/

NFC/grapheme behavior comes from the locked unicode-normalization and
unicode-segmentation dependencies under their own terms. No NFKC, global
casing rewrite, learned model, private dictionary, account lookup or network
address fetch is introduced.

The [policy-contract catalog](../../tests/fixtures/policy-contract.json) orders
authored text expectations from logical group files, shared across Rust and
installed-Python consumers. Fingerprints cover the catalog and all referenced
inputs. The original [corpus](../../benches/corpus.json) and
[intent corpus](../../benches/intent-corpus.json) remain frozen; expanded policy
coverage is separate measurement data, not a replacement baseline.

Semantic fixtures protect the normalization contract, not a promise of older
behavior forever. Deliberate reading changes are recorded in the
[changelog](../../CHANGELOG.md). The financial and IBAN examples are selected
public test inputs, not account verification or native-language/voice sign-off.
