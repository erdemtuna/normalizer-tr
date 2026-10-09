# Fallback policy

`ambiguity_policy="fallback"` / `AmbiguityPolicy::Fallback` opts into
source-faithful rendering. Existing valid readings and hints retain precedence.
Preserve remains the default. Fallback does not relax Reject's strictness or
mask engineering errors.

This guide follows repository behavior; see the [changelog](../CHANGELOG.md)
for released and unreleased changes.

The policy prefers clear source formats, then literal letters/digits/symbols,
then conventional Unicode `U+...` identifiers spoken character by character.
It does not repair logical errors, strip controls, invent abbreviation meanings,
reinterpret bare Roman letters or infer mathematics/ordinals from punctuation.
Unknown symbols in otherwise-verbatim text are covered too.

Successful fallback returns `complete=true`, no unresolved `issues`, and
separate immutable `fallbacks` diagnostics with original UTF-8 ranges. The
`fallback_used` getter/property is derived from that collection. Completion
does not certify factual truth, account/checksum validity, privacy or model
vocabulary/pronunciation compatibility.

Fallback-generated segments have kind `Fallback` / `"fallback"` and a
`source.fallback` rule identifier. The strategy is carried in the diagnostic.
Invalid source-shaped dates or identifiers are never labelled as validated
calendar/checksum results.

| Source | Fallback reading | Strategy |
|---|---|---|
| `1.234` | `bin iki yüz otuz dört` | Preferred number |
| `00042` | `sıfır sıfır sıfır dört iki` | Literal |
| `IV` | `ı ve` | Literal, not a Roman numeral guess |
| `10-15` | `on tire on beş` | Literal, not an inferred range |
| `40.03.2026` | `kırk Mart iki bin yirmi altı` | Surface date, still invalid |
| `saat 25:70` | `saat yirmi beş yetmiş` | Surface time, still invalid |
| `12:30` | `on iki otuz` | Preferred clock format, validated components |
| `hello🙂` | `hello gülümseyen yüz` | Named symbol |
| `🫠` | `unikod u artı bir fe a e sıfır` | Unicode code point U+1FAE0 |

An invalid month is not given an invented month name; its notation is literal.
An invalid suffix is not repaired: the delimiter and suffix remain literal
parts. Account/identifier digits are spelled, including zeros, without checksum
certification. Existing decimal, money, telephone and electronic readings do
not change when they already succeed.

Primary coverage is shared by all three policies. Valid compact quantities,
space-grouped money and supported units receive primary readings under Fallback
without fallback records. See the [normalization reference](normalization.md).
The contract requires malformed compounds to own their full source span and
carry the original failure category; their literal output is not a valid-money/
unit certificate. The fixed ownership cases are covered under
[reviewed boundary regressions](normalization.md#reviewed-boundary-regressions-unreleased).

Approved initialisms and suffix-selected pronunciations are primary readings in
every policy, without fallback records: `SGK’ya` -> `se ge kaya` and
`PDF’ten` -> `pe de eften`. An unapproved suffix remains an unresolved whole
expression; Fallback spells its written letters and suffix literally, not using
an inferred pronunciation to repair it.

Exact approved foreign names likewise use primary Pronunciation segments,
without fallback records: `Claude` -> `klod`, `ChatGPT'ye` -> `çet ci pi tiye`.
These are text aliases, not phonemes or model-quality guarantees. A recognized
name with an invalid suffix (`Claude'ye`) receives literal Fallback with
attempted class `Pronunciation`, its original issue category and the whole
source range, not a repaired name reading. See
[approved foreign names](normalization.md#approved-foreign-names) for exact coverage.

Unknown uppercase prose such as `ABC` is preserved rather than spelled.
Identifier `AB12` and uncued Roman `IV` use their protected/literal
readings. Bare or invalid slash-date notation remains literal, without a
PreferredDate or SurfaceDate assumption.

In failed quantity expressions, approved labels are recognized in numeric
context before symbol/letter splitting: `01 °C` reads
`sıfır bir derece Santigrat` with an InvalidForm diagnostic, not a validated
Unit segment. Bare `V`/`GB` do not gain unit meanings.
Paired quote/list punctuation in unclaimed prose gaps remains punctuation.

## Diagnostics and serialization

Rust exposes `fallbacks()` and derived `fallback_used()`. Each immutable record
contains `range`, `attempted_class`, `reason`, `original_category` and `strategy`.
The original category is `None` for symbols which primary recognition never
claimed. Source values are not duplicated in diagnostics.

Optional Rust Serde serializes ranges as `{"start":...,"end":...}` and enum
variants in Rust casing, for example `"SurfaceDate"`. It includes a computed
`fallback_used` field. The Python facade uses frozen dataclasses, flat
`start_byte`/`end_byte` fields and snake-case labels such as `"surface_date"`.
`dataclasses.asdict()` includes stored fields; `fallback_used` is a property,
so explicitly add it if needed when exporting Python records.

Unreleased name recognition adds `SegmentKind::Pronunciation` for successful
primary aliases and `FallbackClass::Pronunciation` for failed name forms.
Serde uses `"Pronunciation"` for either enum value; Python uses
`"pronunciation"` for segment `kind` and fallback `attempted_class`.
Fallback output still has segment kind `"Fallback"` / `"fallback"` and rule
`source.fallback`; only successful primary aliases use `pronunciation.name`.
Both Rust variants were appended, preserving existing enum/Serde tag order;
that does not preserve source compatibility for exhaustive Rust matches.
Update those matches and strict serialized allowlists as described in the
[migration notes](../CHANGELOG.md#migration).
Record fields, other labels and the derived fallback boolean are unchanged.

No `fully_rendered` flag or second independently stored fallback boolean exists.
All input, hint, cancellation/deadline and resource checks apply.
The 512 KiB logical result budget counts fallback records and both owned text
copies; amplified literals may fail with a real result-limit error.

The base Fallback policy is available since 0.4.0; the expanded initialism/name
coverage remains Unreleased. Fallback support incorporates parts of
[Canberk's contribution](https://github.com/erdemtuna/normalizer-tr/pull/1).
Internal responsibility boundaries are in [CONTRIBUTING](../CONTRIBUTING.md#code-layout).
