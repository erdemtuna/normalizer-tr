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

| Module | Responsibility |
|---|---|
| `src/api` | Public contract behind crate-root re-exports |
| `src/classify` | Fixed reader priority, validation and whole-span claims; `scan` separates tokenization/indexing, punctuation and signals |
| `src/domain` | Validated numeric, quantity, temporal, electronic and identifier values |
| `src/domain/lexicon` | Approved abbreviation/name catalogs and shared compile-time lookup indexes; typed spoken tails remain in the domain |
| `src/notation` | Shared written-form helpers, explicit character names and noncertifying source surfaces |
| `src/interpretation.rs` | Neutral findings retaining their family, original issue and prevalidated alternatives |
| `src/resolution` | Static policy selection; `fallback` renders alternatives, ordered literals and code-point strategies with bounded emission |
| `src/numerals.rs`, `src/morphology.rs`, `src/verbalize.rs` | Exact rendering and spoken-tail metadata |
| `src/morphology/nominal.rs` | Private bounded name inflection, current-stem transitions and borrowed suffix matching |
| `src/morphology/nominal/tests.rs` | Nominal unit goldens, state transitions, invalid tails and finite-form consistency |
| `src/source_map.rs` | NFC recognition mapped to original UTF-8/grapheme coordinates |
| `src/resources.rs` | Immutable patterns validated and cached once, not input/result memoization |
| `src/pipeline.rs` | Source composition, diagnostics, adjacency padding and allocation budgets; aggregate Reject after engineering checks |

Classification/domain code does not depend on the renderer. Source date/time
surfaces do not relax validated domain constructors. Borrowed literal parts
share exact numeric and approved-label helpers; the fallback emitter checks
controls and capacity during traversal. `src/classify/symbols.rs` supplements
only unclaimed graphemes in Fallback. Python calls this engine without duplicating
language rules.

`src/domain/lexicon/abbreviations.rs` is the abbreviation catalog entry point.
It owns default and alternate Lexemes with typed source/target tails. The parent
lexicon validates suffixes and selects a reading; scanners use approved bases
only for source boundaries. Keep new names out of scanner, rendering and Python
special cases. Definitions live in thematic `titles`, `civic`, `education`,
`finance` and `technology` modules; maintain them in logical order, not alphabetical
order. `src/domain/lexicon/static_index.rs` combines the groups at compile time
for allocation-free binary lookup.
Test global key uniqueness, index coverage and unambiguous variant suffixes.

`src/domain/lexicon/pronunciations.rs` owns the exact foreign-name catalog,
grouped into `ai`, `developer` and `consumer`. It shares the static-index helper;
bounded phrase candidates are borrowed from that index. Keep approved spellings,
text aliases and typed suffix tails together. `src/classify/readers/pronunciation.rs`
claims the longest exact phrase after whole electronic recognition, without
guessing casing or meanings. Test all approved case families, invalid suffix
ownership, mixed prose, protected identifiers, phrase boundaries and original
Unicode coordinates. Text goldens are not speech-model/audio validation.

The scanner indexes numeric-run ends once for bounded money lookahead and
distinguishes paired quotation boundaries from suffix apostrophes. Keep
recognition separate from strict value validation and share typed label metadata.
Its incremental piece signals and complete tight-list admission stay separate
from domain parsing and rendering. Rejecting a group must not invent candidates
or hide genuine unresolved findings; source admission belongs with boundary claims.

## Change rules and tests

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

`tests/fixtures/policy-contract.json` is an ordered catalog of logical groups
under `tests/fixtures/policy-contract/`. Rust tests and benchmarks share a
compiled fixture loader; installed-Python tests and verification consume the
same catalog. Add cross-feature cases and exercise Preserve, Reject and Fallback
independently. Do not alter the frozen benchmark inputs/order to improve an aggregate.
Append new groups and retain existing case IDs and order. Change expected
outputs only for intentional behavior changes documented in the changelog.
`tests/support/policy_contract.rs` supplies the Rust loader; Python consumers
flatten the files in catalog order and fingerprint every input. The report key
`expanded_coverage_measurement` is retained for compatibility, not a second
fixture suite. Fixture origins and rights are recorded in
[data provenance](src/data/provenance.md).

Name matching and rendering share typed suffix transitions. Reuse them rather
than deriving harmony from English spelling or duplicating rules in readers
or Python. Initialization checks the catalog once and returns
`InvalidConfiguration` for inconsistent definitions. Different grammatical
analyses can have the same spoken result; tests should accept that agreement
and reject conflicting outputs.

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

## Full verification

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

## Releasing

Maintain [CHANGELOG.md](CHANGELOG.md): leave changes under Unreleased until the
next version and release date are agreed. Before publishing, update core/binding
versions consistently and finalize that entry without rewriting past releases.
Mark preparation-only entries clearly, then finalize release wording before
tagging. Versioned source and build artifacts do not imply registry availability;
confirm each registry's publication independently.
Platform requirements are in the [Python guide](bindings/python/README.md#install).

[release.yml](.github/workflows/release.yml) is manually dispatched. Its default
is build/test only: 16 wheels, a tested source distribution, the core archive and
an exact checksum manifest. No PR or ordinary push publishes packages.

PyPI publication requires a matching `v<version>` tag, `publish=true`, the `pypi`
environment and its configured Trusted Publisher. Only the publish job has OIDC
permission; no persistent PyPI token is stored.

Publish the core to crates.io using owner-configured Cargo authentication, from
the same clean revision and byte-identical reviewed archive. The internal
Rust/Python companion keeps `publish = false`. Verify fresh registry consumers
and live checksums afterward.

Never paste credentials into chat, command arguments or source. If an upload has
an uncertain result, inspect the registry before retrying. Do not reuse a version
for different artifacts. Registry publishing is owner-controlled after
built-artifact verification, never an untrusted PR action.

Owned contributions are Apache-2.0; retain third-party notices. Keep credentials,
models, integrations, private history, audio, wheels, environments and reports
outside the public source/packages and Git.
