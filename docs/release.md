# Release notes

## Unreleased

The Rust core now recognizes compact quantities, supported spaced currency
symbols, validated space-grouped money, additional percent positions and
spaced contextual ranges. Number/label/suffix validation remains exact and
malformed compounds stay whole.

Adds numeric-context `°C` (`derece Santigrat`), `V`, `kW`, `kWh` and `GB`, their
validated existing case families, and explicit capitalization aliases for
doctor/professor/doçent titles. Supports fixed `DD/MM/YYYY` only with a date cue
or whole Date hint; bare/invalid slash notation remains literal under Fallback.
Clock seconds are still outside coverage.

Smart double quotes, clearly paired single quotes and unambiguous quantity-list
boundaries preserve punctuation without confusing decimal commas or suffix
apostrophes. Unknown uppercase prose such as `ABC` is now verbatim in every
policy; protected identifiers and recognized Roman-looking notation keep their
existing rules.

Public signatures, result/diagnostic shapes, policy defaults and limits are
unchanged, but these are deliberate behavioral changes: previously unresolved
forms may now resolve, Reject may succeed, and fallback counts/readings can
change. A fragment hint inside a newly recognized compound is invalid.
Update output snapshots and review diagnostic-dependent consumer logic.
No spelling-override hint, global casing/whitespace rewrite, rounding, currency
conversion or dependency is introduced.

New shared Rust/Python goldens cover every policy. Performance reporting adds
separate reviewed new-coverage and scaling measurements while retaining the
original frozen corpora and their measurement rules.

## 0.4.0

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
See the [fallback contract](fallback.md).

The README is shorter, with API details in dedicated guides.

## 0.3.0

First registry release of the shared Rust engine and typed Python binding.
Both distributions are named `normalizer-tr`, imported as `normalizer_tr`.

## Compatibility

Rust 1.94 or newer; release builds use 1.99.0. Ordinary CPython 3.11 to 3.14
wheels target Windows/Linux x64 and macOS x64/arm64. Linux requires glibc 2.28
or newer; macOS requires 12.0 or newer. Other interpreters and architectures
are not promised.

## Maintainer release process

`release.yml` is manually dispatched. Its default is build/test only:
16 wheels, a tested source distribution, the core archive and an exact checksum
manifest. No PR or ordinary push publishes packages.

PyPI publication requires a matching `v<version>` tag, `publish=true`, the `pypi`
environment and its configured Trusted Publisher. Only the publish job has
OIDC permission. No persistent PyPI token is stored.

Publish the core to crates.io using owner configured Cargo authentication,
from the same clean revision and byte identical reviewed archive. The internal
Rust/Python companion keeps `publish = false`. Verify fresh registry consumers
and live checksums afterward.

Never paste credentials into chat, command arguments or source. If an upload
has an uncertain result, inspect the registry before retrying. Do not reuse a
version for different artifacts. Models, integrations, private history and audio
remain outside the public source and packages.
