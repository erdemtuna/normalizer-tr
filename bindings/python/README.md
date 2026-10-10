# Python binding

Distribution `normalizer-tr`; import `normalizer_tr`, native submodule
`normalizer_tr._native`. This is the Python bridge to the Rust engine, not a
second implementation. This guide follows the repository API; see the
[changelog](../../CHANGELOG.md) for release status and migration notes.
The curated initialism/name additions and `"pronunciation"` labels below are
included in **0.5.0**; review the migration notes before upgrading.

## Install

```text
python -m pip install normalizer-tr
```

Wheels support ordinary CPython 3.11–3.14 on Windows/Linux x64 and macOS
x64/arm64. Linux requires glibc 2.28+; macOS requires 12.0+. Release wheels are
built with Rust 1.99.0, but compatible wheels need no Rust compiler to install.
ABI3, PyPy, free-threaded Python and other architectures are not promised.
Import/use needs no Torch, model folder, credentials, network or Python
normalization subprocess.

## Build from source

Source builds require Rust. On Windows, also install the MSVC C++ build tools.
From a repository checkout on Windows, in PowerShell:

```powershell
py -3.13 -m venv .venv
.\.venv\Scripts\python.exe -m pip install .\bindings\python
.\.venv\Scripts\python.exe -X utf8 .\bindings\python\examples\showcase.py
```

This builds a wheel locally. To install an existing compatible wheel, use
`python -m pip install --no-deps <wheel-path>`. Only source builds require Rust.

## API

```python
from normalizer_tr import Normalizer, Hint, NormalizationError, NORMALIZER_ID
n = Normalizer()
assert n.normalizer_id == NORMALIZER_ID
r = n.normalize("25 TL; 1.234")
assert r.normalized_text == "yirmi beş Türk lirası; 1.234"
assert not r.complete
assert (r.issues[0].start_byte, r.issues[0].end_byte) == (7, 12)
assert n.normalize("IV", hints=(Hint(0, 2, "roman"),)).normalized_text == "dört"
try:
    n.normalize("1.234", ambiguity_policy="reject")
except NormalizationError as error:
    assert error.code == "unresolved"
    assert error.issues
```

`Normalizer()` is the only constructor; there is no selector argument or older
behavior mode. `normalizer_id` is diagnostic package/build metadata, not a
configuration knob. Result properties: normalized_text, locale, normalizer_id,
complete, immutable tuples of frozen segments/issues/fallbacks, and the derived
`fallback_used` property. Segment fields:
start_byte/end_byte/kind/text/rule_id. Issue fields:
start_byte/end_byte/category/explanation. All offsets are half-open
**original UTF-8 bytes**, not Python character indices.

The API accepts `ambiguity_policy="fallback"` as well as
`"preserve"` (default) and `"reject"`. It keeps resolved readings unchanged,
renders otherwise unresolved notation and symbols, and returns `complete=True`
with separate handled-source diagnostics rather than unresolved issues:

```python
r = n.normalize("00042; hello🙂", ambiguity_policy="fallback")
assert r.normalized_text == "sıfır sıfır sıfır dört iki; hello gülümseyen yüz"
assert r.complete and r.fallback_used and not r.issues
```

Approved initialisms and exact foreign names use primary readings in every
policy, not fallback records:

```python
r = n.normalize("SGK'ya; GitHub Copilot'ın", ambiguity_policy="reject")
assert r.normalized_text == "se ge kaya; git hab ko paylıtın"
assert r.complete and not r.fallback_used
assert r.segments[-1].kind == "pronunciation"
assert r.segments[-1].rule_id == "pronunciation.name"
```

Aliases are Turkish-readable text, not phonemes. For exact casing, phrase and
suffix boundaries, see [approved foreign names](../../docs/normalization.md#approved-foreign-names).
Invalid suffixes are not repaired; handled name failures have fallback
`attempted_class="pronunciation"`. Update strict kind/class allowlists when
adopting the 0.5.0 API.

Unreleased name handling includes `iPhone'umdan` -> `ayfonumdan` under all
policies, without fallback diagnostics. The Python call and result shapes are
unchanged. See [supported suffixes](../../docs/normalization.md#nominal-suffixes-unreleased)
for the shared Rust grammar and the
[Unreleased migration notes](../../CHANGELOG.md#unreleased) for changed
Instagram/WhatsApp readings and previously accepted spellings that now fail.

`FallbackDiagnostic` is frozen and has start_byte/end_byte/attempted_class/
reason/original_category/strategy. Completion is not logical-value validation,
redaction or a voice-quality promise. See the [fallback contract](../../docs/fallback.md)
for readings and Rust Serde versus Python record naming, and the
[normalization reference](../../docs/normalization.md) for supported formats.

`normalize(text, *, ambiguity_policy="preserve", hints=(), cancellation=None,
deadline_ms=None)` takes Python str, typed Hint objects, optional
CancellationToken, and an integer deadline of 1–60000 ms. Booleans do not count as
integer coordinates/deadlines; surrogates are invalid input. Call-shape errors
are TypeError/ValueError, never partial success. Hint kinds:
cardinal/digits/date/time/ordinal/roman/range/telephone/electronic.
Whole-span/grapheme/overlap/content checks are authoritative in Rust.

`NormalizationError` has code, immutable issues and limit_kind. Codes:
invalid_input, invalid_hint, invalid_configuration, limit_exceeded, cancelled,
unresolved, internal. Strict errors contain issues, no normalized result.
CancellationToken.cancel() signals associated ongoing/later calls. Native
work executes pure Rust detached from the interpreter after copying/validating
arguments; records marshal afterward.

Deadlines/cancellation are cooperative, not hard call-duration limits; see
[reviewed boundary regressions](../../docs/normalization.md#reviewed-boundary-regressions-050).

## Development and verification

See [CONTRIBUTING](../../CONTRIBUTING.md#set-up) for pinned development tools
and installed-wheel tests. To build a release wheel after installing those tools:

```powershell
.\.venv\Scripts\python.exe -m maturin build --release --locked --manifest-path .\bindings\python\Cargo.toml --interpreter .\.venv\Scripts\python.exe --out .\target\wheels
```

Follow the [full verification procedure](../../CONTRIBUTING.md#full-verification)
for package inspection, clean-environment consumers, audits and measurements.

The wheel contains facade/stubs/py.typed/native binary/SBOM and complete owned/
upstream notices only. No speech integration, model weights, network code or credentials.
External code/model redistribution rights remain separate and unestablished.
