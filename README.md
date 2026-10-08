# normalizer-tr

[![CI](https://github.com/erdemtuna/normalizer-tr/actions/workflows/ci.yml/badge.svg)](https://github.com/erdemtuna/normalizer-tr/actions/workflows/ci.yml)

Turkish text normalization for speech, with a Rust core and a typed Python API.
Convert numbers, money, dates, measurements and other supported notation into
spoken Turkish while preserving ordinary prose. Runs offline, without a speech
model or service. Decimal and money arithmetic is exact.

```text
saat 09:30'da 5 kg malzeme ve %12,5'lik fark
→ saat dokuz otuzda beş kilogram malzeme ve yüzde on iki virgül beşlik fark
```

## Python

```sh
python -m pip install normalizer-tr
```

```python
from normalizer_tr import Normalizer

normalizer = Normalizer()
result = normalizer.normalize("25 TL; 5 kg")
assert result.normalized_text == "yirmi beş Türk lirası; beş kilogram"
assert result.complete
```

Wheels support CPython 3.11 to 3.14 on Windows and Linux x64, and macOS
x64 and arm64. Compatible wheels need no Rust compiler.
[Platform requirements and source builds](https://github.com/erdemtuna/normalizer-tr/blob/main/bindings/python/README.md).

## Rust

Requires Rust 1.94 or newer. Add to `Cargo.toml`:

```toml
[dependencies]
normalizer-tr = "0.4"
```

```rust
use normalizer_tr::{NormalizeOptions, Normalizer};

let normalizer = Normalizer::new()?;
let result = normalizer.normalize("25 TL; 5 kg", &NormalizeOptions::default())?;
assert_eq!(result.normalized_text(), "yirmi beş Türk lirası; beş kilogram");
assert!(result.complete());
# Ok::<(), normalizer_tr::NormalizeError>(())
```

Reuse a `Normalizer` across calls. It is cloneable and `Send + Sync`.
Enable the optional `serde` feature to serialize results.

## Choose how to handle ambiguity

| Policy | Behavior |
|---|---|
| `preserve` (default) | Keep unresolved spans as written. Return `complete=false` and `issues`. |
| `reject` | Return an error if any span is unresolved. |
| `fallback` | Render unresolved notation and symbols. Return handled assumptions in `fallbacks`. |

```python
result = normalizer.normalize("1.234; AB12", ambiguity_policy="fallback")
assert result.normalized_text == "bin iki yüz otuz dört; a be bir iki"
assert result.complete and not result.issues
assert result.fallback_used
```

In Rust, set `NormalizeOptions.ambiguity_policy` to
`AmbiguityPolicy::Fallback` or `AmbiguityPolicy::Reject`.

Fallback prefers clear formats, then literal readings and named symbols, then
spoken Unicode codes. It does not correct invalid facts or certify identifiers.
Check `fallbacks` when those assumptions matter. Invalid input or hints,
cancellation and resource limits remain errors in every policy.

Hints provide explicit intent. All hint and diagnostic ranges are original
UTF-8 byte offsets, not character positions.

## Documentation

| Guide | Contents |
|---|---|
| [Rust API](https://docs.rs/normalizer-tr) | Types, methods and examples |
| [Python API](https://github.com/erdemtuna/normalizer-tr/blob/main/bindings/python/README.md) | Hints, errors, cancellation and building |
| [Normalization reference](https://github.com/erdemtuna/normalizer-tr/blob/main/docs/normalization.md) | Supported formats and boundaries |
| [Fallback](https://github.com/erdemtuna/normalizer-tr/blob/main/docs/fallback.md) | Reading strategies and diagnostics |
| [Contributing](https://github.com/erdemtuna/normalizer-tr/blob/main/CONTRIBUTING.md) | Setup, architecture and verification |

This is a pre 1.0 library with bounded coverage, not a universal pronunciation
engine. See [performance](https://github.com/erdemtuna/normalizer-tr/blob/main/PERFORMANCE.md) for measurements and
[release notes](https://github.com/erdemtuna/normalizer-tr/blob/main/docs/release.md) for changes.

Current source also supports compact quantities (`5kg`, `25TL`), validated
space-grouped money and intent-gated slash dates. Unknown uppercase prose is
preserved, not automatically spelled; Roman-looking notation and identifiers
keep their separate rules. See the normalization reference for exact boundaries.

## License

[Apache-2.0](https://github.com/erdemtuna/normalizer-tr/blob/main/LICENSE),
with [third party notices](https://github.com/erdemtuna/normalizer-tr/blob/main/THIRD_PARTY_NOTICES.md).

Thanks to [@canberk7](https://github.com/canberk7) for his contribution to fallback support.
