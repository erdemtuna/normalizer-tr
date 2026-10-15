# Changelog

Entries describe versioned changes and migration requirements.
For publishing instructions, see [CONTRIBUTING](CONTRIBUTING.md#releasing).
Platform requirements are in the [README](README.md) and
[Python installation guide](bindings/python/README.md#install).

## Unreleased

## 0.8.0 - 2026-10-15

### Behavior and migration

Changes since 0.7.0. Registry availability is verified separately from versioned
source and build artifacts.

- Keep unsupported suffixed telephone runs atomic under Preserve, Reject and
  Fallback, including established malformed/incomplete runs and invalid
  separators. `0532 123 45 67'den` now has one invalid owner over `[0,18)`,
  instead of a successful suffixed Cardinal escaping its shortened owner.
  Fallback spells the whole owner literally with one Identifier/InvalidForm
  diagnostic. Valid unsuffixed telephone pronunciation and neighboring fields
  remain unchanged; telephone suffixes and invalid separators are not accepted.
  Migration: update text/segment/diagnostic snapshots and original byte ranges.
  Previously accepted component hints such as `Hint(0,11,"digits")` and
  `Hint(12,18,"cardinal")` now fail with `invalid_hint`, as required by whole-owner
  validation. Public types, serialized labels, policy defaults and limits are
  unchanged. No audio validation was performed.
  Mixed hyphen groups, adjoining phone parentheses and non-composing tail
  marks also retain one owner; phone-local closers do not escape as raw text,
  and outer prose wrappers remain independent.
  Marked closing-parenthesis graphemes no longer swallow the following token
  or cause an extended claim to skip it. Adjacent identifiers/quantities/names
  retain their own owners; standalone `)\u030112` now retains the closer and
  reads the independent number. Marked semicolon/comma compounds are unchanged.
  Repeated/empty tails after owned closers stay atomic and cannot quote away
  the suffix of a later independent phone.
  Empty-tail numeric compounds preserve confirmed outer parentheses without
  stripping unmatched invalid local closers; pairing is indexed lazily per call.

Supported standalone uppercase letters now receive primary Turkish letter names.
This applies to ordinary capitals and unmarked initials, not just technical
phrases. Default Preserve does not disable these primary readings.

| Input / policy | Previously | Now |
|---|---|---|
| `P ve S dalgaları`, every policy | Unchanged prose | `pe ve se dalgaları` |
| `C vitamini`, Preserve | Unchanged, incomplete, with an issue | Primary `ce vitamini`, complete without issues |
| `C vitamini`, Reject | Unresolved error | Primary `ce vitamini` |
| `C vitamini`, Fallback | `ce vitamini` with a Roman literal-fallback record | Same text, but primary Symbol / `letter.name`, without fallback records |
| `S Yılmaz`, every policy | Initial unchanged | `se Yılmaz` |

Public call shapes, fields, enum labels, hints, defaults and limits are unchanged,
but reading and diagnostic semantics change. Consumers must not assume that
Symbol excludes letters, or that a formerly literal Roman reading still appears
in `fallbacks`. Even A/O receive primary Symbol segments rather than Verbatim
provenance. Long singleton runs can now reach existing candidate/result budgets.
Lowercase, unknown uppercase words, arbitrary punctuation fragments and protected
compound owners are not newly spelled. Multi-letter Romans, contextual numeral
readings and valid explicit hints retain precedence. See
[standalone-letter boundaries](docs/normalization.md#standalone-uppercase-letters).

Isolated paired quotes now also support interior edge whitespace and unambiguous
nesting, while preserving wrappers and original UTF-8 ranges. This corrects more
than letter readings: `' SGK'ya'` becomes `' se ge kaya'`, and `' 25 TL'` becomes
`' yirmi beş Türk lirası'`, rather than leaving their contents unresolved or
speaking a closing quote as `kesme` under Fallback. Truly invalid contents remain
atomic, but diagnostic ranges/categories and literal output can change when the
wrapper is excluded. In `' 01 V'`, the quantity remains invalid at `[2,6)`.
Review stored source ranges as well as output text. See
[quote boundaries](docs/normalization.md#quotation-and-list-boundaries).

These changes concern normalized text and public result semantics; no
speech-model or audio-quality improvement is claimed.

### Performance and known limits

The phone-ownership fix includes an explicitly accepted measured cost on
equivalent malformed 4-32 KiB name-suffix workloads. The
[accepted risk and source-specific evidence](https://github.com/erdemtuna/normalizer-tr/pull/40#issuecomment-6176623522)
retain the absolute latency deltas and failed-work distinctions. Those
measurements are not a comparison of every 0.7.0/0.8.0 workload or a universal
latency, hard-deadline, peak-memory or no-regression guarantee.

### Development tooling and maintenance

- Separate telephone source ownership from IBAN coordination and compound
  scanning from the common token/index pass. Group telephone acceptance tests
  by their contract purpose; inputs, reading/policy behavior and public APIs
  are unchanged by this extraction.
- Separate serial performance orchestration, validated comparisons and Markdown
  presentation into focused modules. Register the extracted modules for
  latest-push performance evidence selection.
- Separate generic quote framing from raw atom admission with typed quote kinds.
  This refactor preserves the behavior described above.
- Validate native comparison reports against their respective source fixture
  catalogs, allowing append-only regression coverage without misvalidating
  an older baseline against newer inputs. Changed cohorts remain non-comparable.
- Include long-input and failed work in performance comparisons, with exact
  input/options, full outcomes and explicit non-comparability. Native and
  installed-Python reports require version two; regenerate older artifacts
  instead of adapting them. Public library APIs, behavior and limits are unchanged.
- Record the requested and measured baseline revisions explicitly. For the
  known version-two instrumentation transition, hosted PR comparisons use the
  frozen instrumented reference and run the complete measurements; other PRs
  use their actual base. Public compatibility still checks the actual PR base.
  See [PERFORMANCE.md](PERFORMANCE.md#hosted-pr-evidence) for the policy and usage.
- Exclude Python bytecode caches from the core package's explicit source list,
  so importing benchmark helpers does not break clean-source packaging.
- Label successful and failed long-input work in checked comparisons and stream
  native report serialization through a checked buffered writer. Frozen call
  timing boundaries and full evidence remain unchanged.
- Clarify fallback diagnostic strategies and make the measurement guide focus
  on running and interpreting comparisons. General README navigation follows
  main; release-specific references are explicitly labeled.

## 0.7.0 - 2026-10-11

- Add bounded plural, six-person possession, case, locative-plus-ki and copular
  suffixes for recognized numbers, percentages and quantities. Preserve
  established case readings and report conflicting new spoken analyses rather
  than selecting by parser iteration order.
- Support all date formats' case/copular tails with distinct written/spoken
  ending metadata, automatic year-first slash dates, exact following date cues,
  suffixed dotted clocks and validated dot/colon clock ranges.
- Read flat multiplication chains without evaluating them. Validate whole
  operands, retaining grouped money and excluding protected identifiers.
- Recognize contextual address aliases and atomic fields, numbered streets,
  postal digits/labels and locality separators.
- Correct grouped-account punctuation and continuation ownership, including
  component-hint and invalid-prefix escapes.
- Bound product lookahead at independent address fields, reject mixed arithmetic
  as one expression and exclude complete protected telephone operands.
- Keep grouped numeric ranges and date-plus-dash prose out of clock-range
  admission without weakening invalid clock validation.
- Remove pre-existing quadratic failed-telephone lookahead and electronic
  parenthesis trimming. Preserve policy outcomes and add cooperative checks
  during electronic boundary scanning.

### Migration

Public call shapes, enum variants, serialized labels and record fields remain
unchanged. New primary readings change text, strict-mode success and diagnostic
counts. Composite primary readings can emit multiple existing semantic segments;
whole-owner hints still cannot select their components. Each emitted component
counts toward work/output limits.

Rates deliberately change from `saatte doksan kilometre` to
`doksan kilometre saat` (and the analogous `m/s` order), for bare and suffixed
forms. Date suffixes validate against the last written component and are adapted
to the canonical spoken year. Clock ranges retain endpoint order; they do not
calculate an overnight duration. Address-specific postal labels and separator
readings apply only in reviewed address context.

Corrected source ownership can invalidate formerly admitted component hints and
change fallback spans. Invalid accounts still do not receive Iban certification.
Preserve/Reject/Fallback defaults and real input/control/limit errors remain.
No downstream speech-model validation was performed; this release validates
the library's normalized text, not audio quality.

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
