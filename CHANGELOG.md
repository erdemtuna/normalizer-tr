# Changelog

Unreleased entries describe repository changes not yet available in published
packages. For publishing instructions, see [CONTRIBUTING](CONTRIBUTING.md#releasing).
Platform requirements are in the [README](README.md) and
[Python installation guide](bindings/python/README.md#install).

## Unreleased

- Recognize compact quantities, supported spaced currency symbols, validated
  space-grouped money, additional percent positions and spaced contextual ranges.
  Validation remains exact; malformed compounds stay whole.
- Add numeric-context `°C` (`derece Santigrat`), `V`, `kW`, `kWh` and `GB` with
  validated case suffixes, plus explicit capitalization aliases for doctor,
  professor and doçent titles.
- Support fixed `DD/MM/YYYY` with a date cue or whole Date hint. Bare/invalid
  slash notation remains literal under Fallback; clock seconds remain unsupported.
- Preserve smart double quotes, clearly paired single quotes and unambiguous
  quantity-list punctuation without confusing decimals or suffix apostrophes.
  Unknown uppercase prose such as `ABC` stays verbatim in every policy;
  protected identifiers and recognized Roman-looking notation keep their rules.
- Reorganize internal modules without changing public imports, serialization or
  reading behavior. Keep policy-contract/scaling measurements separate from the
  original frozen benchmark corpora and aggregates.

### Migration

Public signatures, result/diagnostic shapes, policy defaults, limits and
dependencies are unchanged. The expanded recognition is a deliberate behavior
change: previously unresolved forms may resolve, Reject may succeed, and
fallback counts/readings may change. Fragment hints inside recognized compounds
are invalid. Compact strings matching an approved quantity are treated as
quantities, not opaque codes. Review output snapshots, diagnostic-dependent logic
and hint ranges.
No spelling override, global casing/whitespace rewrite, rounding or currency
conversion is added. See the [normalization reference](docs/normalization.md)
for exact boundaries.

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
