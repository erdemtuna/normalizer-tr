# Fallback policy

`ambiguity_policy="fallback"` / `AmbiguityPolicy::Fallback` opts into
source-faithful rendering. Existing valid readings and hints retain precedence.
Preserve remains the default; Reject and all real engineering errors are unchanged.

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
Invalid source-shaped dates or
identifiers are never labelled as validated calendar/checksum results.

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

No `fully_rendered` flag or second independently stored fallback boolean exists.
All existing input, hint, cancellation/deadline and resource checks still apply.
The 512 KiB logical result budget counts fallback records and both owned text
copies; amplified literals may fail with a real result-limit error.

## Responsibility boundaries

Primary readers own precedence, validation and whole-span claims. Typed
unresolved requests retain the family, original issue and available numeric
alternative. Distinct surface types own date/time notation, without relaxing
validated semantic constructors. The finite resolver selects number/surface/
literal/code-point strategies; borrowed ordered literal parts reuse the exact
numeric and approved-label helpers. The bounded emitter checks controls and
capacity during traversal. Gap supplementation owns only unclaimed graphemes.
Pipeline composition owns source coordinates, diagnostics and allocation
accounting. Python performs no linguistic normalization.

Available since 0.4.0. Fallback support incorporates parts of
[Canberk's contribution](https://github.com/erdemtuna/normalizer-tr/pull/1).
