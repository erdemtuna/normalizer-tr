# Changelog

Entries describe versioned changes and migration requirements.
For publishing instructions, see [CONTRIBUTING](CONTRIBUTING.md#releasing).
Platform requirements are in the [README](README.md) and
[Python installation guide](bindings/python/README.md#install).

## 0.6.0 - 2026-10-11

- Support plural, all six possessive forms and a final case suffix for every
  exact approved foreign name. `iPhone'umdan` reads `ayfonumdan`, and
  `iPhone'larımıza` reads `ayfonlarımıza`. See the
  [grammar and examples](docs/normalization.md#nominal-suffixes).
- Replace Instagram's `instıgrem` reading with `instagram` and the
  WhatsApp/Whatsapp reading `vats ep` with `vatsap`. `Instagram'a`,
  `Instagram'da` and `WhatsApp'tan` receive primary readings under every policy.
  There is no older pronunciation option or legacy suffix alias.
- Validate the pronunciation catalog once during initialization. Reject
  inconsistent definitions before normalization. Suffix matching uses borrowed
  fragments without allocating candidate outputs.
- Add name suffix regression cases and scaling workloads to the shared
  measurements. The original benchmark corpora and report keys are unchanged.

### Migration

Public signatures, enum values, serialized labels, record fields, hint kinds,
policy defaults, limits and dependencies are unchanged. The new grammar applies
only to exact approved names, not numbers, quantities, units or initialisms.
Existing casing, phrase ownership, source ranges and protected contexts remain
unchanged.

Bare Instagram and WhatsApp outputs change. Newly accepted suffixes no longer
produce unresolved issues or fallback diagnostics, and Reject can succeed.
Spellings based on the old readings, such as `Instagram'e`, `Instagram'de` and
`WhatsApp'ten`, no longer validate: Preserve retains the whole source with an
issue, Reject errors, and Fallback spells it literally. Update text snapshots
and code that depends on diagnostics. Malformed suffixes are not repaired.

Independent EMA Lightning listening validation was not performed; the release
owner waived that gate. Passing text tests and benchmarks does not establish
audio quality.

## 0.5.0 - 2026-10-09

- Remove repeated numeric-prefix scans from comma-boundary detection and literal
  fallback. Retain valid later numeric readings and cooperative control checks.
- Keep attached duplicate currency markers in one invalid prefixed amount rather
  than accepting separate money fragments.
- Evaluate safely separated semicolon members independently and validate complete
  tight comma groups before primary abbreviation/name admission. Keep ordinary
  spaced prose and genuine protected-source diagnostics unchanged.
- Add 65 reviewed uppercase initialisms (80 approved abbreviation spellings in
  total, including existing aliases) with typed pronunciation tails. `SGK`
  defaults to `se ge ka`; approved suffixes select `ke`/`ka` alternatives and
  PDF's `fe`/`ef` alternative without silently canonicalizing the reading.
  API/IP/HDMI use an explicit international `i` reading.
- Recognize guarded no-space abbreviation lists and trailing colon/period
  punctuation while protecting unknown prose, identifiers, electronic text and
  canonical Roman intent.
- Recognize compact quantities, supported spaced currency symbols, validated
  space-grouped money, additional percent positions and spaced contextual ranges.
  Validation remains exact; whole-span ownership is the intended contract.
- Add numeric-context `°C` (`derece Santigrat`), `V`, `kW`, `kWh` and `GB` with
  validated case suffixes, plus explicit capitalization aliases for doctor,
  professor and doçent titles.
- Support fixed `DD/MM/YYYY` with a date cue or whole Date hint. Bare/invalid
  slash notation remains literal under Fallback; clock seconds remain unsupported.
- Preserve smart double quotes, clearly paired single quotes and unambiguous
  quantity-list punctuation without confusing decimals or suffix apostrophes.
  Unknown uppercase prose such as `ABC` stays verbatim in every policy;
  protected identifiers and recognized Roman-looking notation keep their rules.
- Add 35 reviewed foreign-name speech aliases with 47 exact keys, including
  `Claude`, `ChatGPT`, `GitHub Copilot` and `Visual Studio Code`. They are primary
  readings under every policy, with explicit case aliases, bounded phrases and
  validated Turkish case suffixes; unknown spellings are not guessed.
  `EMA Lightning`, `ema lightning` and `ema-lightning` read as `ema laytning`.
- Add `SegmentKind::Pronunciation`, `FallbackClass::Pronunciation`, Python
  `"pronunciation"` labels and primary rule ID `pronunciation.name`.
- Reorganize internal modules by responsibility without changing public import
  paths or existing enum/Serde tags. Keep thematic lexical definitions in logical
  order with shared compile-time static indexes for borrowed lookup.
- Expand the ordered policy-contract catalog to 179 cases; the earlier 170 remain
  an exact prefix. Rust tests/benchmarks share its loader, and Python
  tests/measurements/verification cover every catalog input. Keep these
  measurements separate from the original frozen benchmark corpora and aggregates.

The previously reviewed ownership, list-admission and numeric-scanning gaps are
covered by [review regression tests](docs/normalization.md#reviewed-boundary-regressions-050),
not treated as supported syntax or relaxed contracts.

### Migration

Core, private Rust/Python companion and Python distribution versions are 0.5.0.
Public signatures, result/diagnostic fields, options, policy defaults, limits
and dependency pins are unchanged. The new enum variants were
appended without reordering existing values. Adding variants to these exhaustive
public enums is a **source-breaking change for existing exhaustive Rust matches**,
which must handle `SegmentKind::Pronunciation` and `FallbackClass::Pronunciation`.
Closed serialized schemas/allowlists must also accept `"Pronunciation"`.
Python segment kinds and fallback attempted classes use `"pronunciation"`. See
[diagnostics and serialization](docs/fallback.md#diagnostics-and-serialization).

The expanded recognition is a deliberate behavior
change: previously unresolved forms may resolve, Reject may succeed, and
fallback counts/readings may change. Fragment hints inside recognized compounds
are invalid. Compact strings matching an approved quantity are treated as
quantities, not opaque codes. Review output snapshots, diagnostic-dependent logic
and hint ranges. Approved initialisms previously left verbatim now receive
primary Abbreviation segments. Their unmatched suffixes produce structured
issues (or Reject errors) instead of being accepted as ordinary prose.
Exact approved foreign names now receive primary Pronunciation segments even
without context: `Apple` and `Rust` are aliases, while lowercase `apple` and
`rust` remain prose. Recognized names with wrong suffixes retain a whole-span
issue under Preserve, fail under Reject, or receive literal Fallback rather than
a repaired alias. Review casing-sensitive snapshots and segment-kind consumers.
No spelling override, global casing/whitespace rewrite, rounding or currency
conversion is added. See the [normalization reference](docs/normalization.md)
for exact boundaries. Speech aliases are Turkish-readable text, not phonemes or
evidence of speech-model quality.

## 0.4.0 - 2026-10-03

Opt into source faithful fallback with Python's `ambiguity_policy="fallback"`
or Rust's `AmbiguityPolicy::Fallback`. Existing valid readings and hints keep
precedence. Preserve remains the default, and Preserve/Reject behavior is unchanged.

Fallback handles ambiguous formats, identifiers and otherwise unhandled symbols.
It returns `complete=true` with separate `fallbacks` diagnostics and a derived
`fallback_used` getter/property. Invalid dates or accounts are read as written,
not repaired or certified. Input/hint errors, cancellation and resource limits
remain real errors.

Rust adds fallback enum variants and diagnostic types. Optional Serde results
now include `fallbacks` and `fallback_used`. Python exposes frozen diagnostic
records; its `fallback_used` property is not a stored dataclass field.
Update exhaustive Rust matches and strict serialized schemas as needed.
See the [fallback contract](docs/fallback.md).

## 0.3.0

First registry release of the shared Rust engine and typed Python binding.
Both distributions are named `normalizer-tr`, imported as `normalizer_tr`.
