# Contributing

Use minimal synthetic examples when reporting bugs: written input, expected
spoken text, actual completeness/issues, options/hints, revision and environment.
Do not include credentials, real account identifiers or private customer text.

## Set up

The release toolchain is Rust 1.99.0; the declared minimum is 1.94 and is tested
separately. The toolchain file includes rustfmt/Clippy. Python release tests
cover ordinary CPython 3.11–3.14 on the selected Windows/Linux/macOS platforms.
On Windows, install the MSVC C++ build tools.

```powershell
git clone https://github.com/erdemtuna/normalizer-tr.git
Set-Location normalizer-tr
cargo test --locked
```

For Python development:

```powershell
py -3.13 -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r .\bindings\python\requirements-dev.txt -c .\bindings\python\constraints.lock
.\.venv\Scripts\python.exe -m pip install .\bindings\python
.\.venv\Scripts\python.exe -m pytest .\bindings\python\tests -q
```

Core/binding development needs no Torch, model files or credentials.

## Code layout

`src/api` groups the public contract behind unchanged crate-root re-exports.
`src/classify` owns fixed reader priority and whole-span claims; its `scan`
modules separate tokenization/indexing, punctuation and source signals.
`src/domain` owns validated numeric, quantity, temporal, electronic and
identifier values. `src/notation` owns shared written-form helpers, explicit
character names and noncertifying source surfaces.
`src/interpretation.rs` holds neutral findings and prevalidated alternatives;
classification/domain code does not depend on the resolver.
`src/numerals.rs`, `src/morphology.rs` and `src/verbalize.rs` share exact
rendering and spoken-tail metadata.
`src/source_map.rs` maps NFC recognition to original UTF-8/grapheme coordinates.
`src/resolution` selects handling with static dispatch; its `fallback` engine
owns alternative rendering, ordered literal parts and bounded emission.
`src/classify/symbols.rs` supplements unclaimed graphemes only in Fallback.
`src/pipeline.rs` composes the source partition, diagnostics and budgets, and
finalizes Reject only after collecting all unresolved issues and checking
engineering failures.
The Python binding calls this engine; it does not duplicate language rules.
The scanner indexes numeric-run ends once for bounded money lookahead and
distinguishes paired quotation boundaries from suffix apostrophes. Keep surface
recognition separate from strict value validation; share typed label metadata
between readers and context-owned fallback parts.

Add positive, negative and mixed-sentence tests. Assert completeness, issue
scope and byte ranges as well as text. Preserve matched-invalid compounds
instead of rewriting their fragments. Reuse exact-number/morphology helpers;
do not introduce unsafe Rust, floating-point money, global lowercasing,
emitted-text reparsing or result caches. Review deliberate reading changes
separately from refactors; do not regenerate fixtures just to pass.
Fallback must not change successful primary readings or Preserve/Reject
outcomes, silently drop symbols, certify invalid dates/accounts, or mask
controls/limit errors as preservation. Add its handled reasons to `fallbacks`,
not unresolved `issues`; assert that completion, provenance and source
partitions agree. See [the fallback contract](docs/fallback.md).

`tests/fixtures/expanded-coverage.json` contains reviewed public examples shared
by Rust, installed-Python tests and separate new-coverage measurements. Add
cross-feature cases and exercise Preserve, Reject and Fallback independently.
Do not alter the frozen benchmark inputs/order to improve an aggregate.

## Checks

```powershell
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo doc --locked --no-deps --all-features
.\.venv\Scripts\python.exe -m ruff check bindings\python benches scripts
.\.venv\Scripts\python.exe -m ruff format --check bindings\python benches scripts
.\.venv\Scripts\python.exe -m mypy bindings\python\python --check-untyped-defs
```

Rebuild/reinstall the binding after native/facade changes before running its
tests. CI also checks default/no-default Rust configurations and installed-wheel
tests, without models.

For full local package/consumer/audit/performance verification, provision:

```powershell
cargo install cargo-audit --version 0.22.2 --locked --root .\target\audit-tool
git clone https://github.com/RustSec/advisory-db.git .\target\advisory-db
```

From a clean committed checkout, use a new absolute output directory:

```powershell
$python = (Resolve-Path .\.venv\Scripts\python.exe).Path
$output = Join-Path $env:TEMP 'normalizer-tr-verification-01'
.\scripts\verify.ps1 -PythonPath $python -OutputDirectory $output
```

Optional `-RustupHome <absolute-isolated-path>` selects an existing isolated
toolchain. Keep the output/environment path short on Windows to avoid
path-length limits. Outputs include logs, packages, consumers and a checksum manifest.
See [PERFORMANCE.md](PERFORMANCE.md) for host-specific timing limits.

Owned contributions are Apache-2.0; retain third-party notices. Keep credentials,
external model source/weights, audio, wheels, environments and reports out of
Git. Only the core can publish to crates.io; the internal Rust/Python companion
keeps `publish = false`. Registry publishing is a separate owner-controlled
release action after built-artifact verification, never an untrusted PR action.
