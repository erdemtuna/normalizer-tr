# Performance and measurement

The development target is warm release Rust **p95 < 1 ms**, separately for
short (1-256) and medium (257-1024) original UTF-8 byte cohorts. The snapshot below
met this host-specific target; it is not a universal SLA, a hosted CI timing
gate or an end-to-end speech-generation guarantee.

The public inputs are in `benches/corpus.json` and `benches/intent-corpus.json`.
Do not shrink the corpus, weaken validation or remove slow cases to claim
improvement. Current semantic outcomes are checked independently by the
characterization and class tests.

Passing those frozen cohorts does not cover every scaling path. The
[reviewed boundary regressions](docs/normalization.md#reviewed-boundary-regressions-050)
also exercise single malformed numeric tokens and cooperative control checks.
Cooperative deadlines are not hard latency guarantees.

## Reproduce

See [CONTRIBUTING.md](CONTRIBUTING.md) to provision the isolated Python tools
and advisory checker/database. From a clean committed Windows checkout:

```powershell
.\scripts\verify.ps1 -PythonPath <absolute-build-python> -RustupHome <optional-isolated-rustup> -OutputDirectory <new-absolute-directory>
```

Omit `-RustupHome` when using the normal installed toolchain. The output path
must be new; reports and packages are never overwritten.

The serial command verifies source/tests/docs, clean packages and external
consumers, then emits `manifest.json` and `summary.txt`. The manifest links
source HEAD, current normalizer identity, compiler/build/host context, corpus/
native-binary hashes, stage results, Cargo VCS and exact package checksums.
It does not commit, publish, run models or change global tools/settings.
Dependent Cargo stages are serial to avoid artifact races.

For the native sampler alone, use a new output path:

```powershell
cargo bench --locked --features serde --bench latency -- --output <new-absolute-report.json>
```

## Native methodology

- `Instant` measures 10,000 individual calls per cohort after 2,000 warmup calls.
- A deterministic mix interleaves Preserve/Reject policy cases; outcomes are
  checked before and after sampling.
- Timings include owned result/error disposal, not just recognition.
- Nearest-rank p50/p95/p99/max and per-class/policy counts are reported.
- Every outlier is retained. Clock overhead is reported, not subtracted.
- Three repetitions are retained by the full verifier; results are not replaced
  by a batched average or confidence interval.

The separate `fallback_measurement` uses the same frozen
cohorts, 2,000 warmups and 10,000 individually timed calls per cohort.
The original Preserve/Reject aggregate/order/corpus are unchanged. Fallback
results are not semantically equivalent to preservation/rejection, so do not
mix their samples into that aggregate or label differences as an optimization.
Python reports follow the same separation, including fallback-only concurrent
calls. No new universal latency promise is made for amplified code-point output.

Policy-contract measurements retain the existing `expanded_coverage_measurement`
JSON key for report compatibility without changing the frozen inputs, original
aggregate/order or existing report fields. Reviewed cases come from
`tests/fixtures/policy-contract.json`, shared with Rust/Python tests.
It is an ordered catalog of money, quantity, lexical, punctuation, date,
unresolved, context and compound-boundary fixture files. Case order and expected
outcomes of existing cases are preserved; initialism, pronunciation-variant and
abbreviation-boundary groups, then AI/developer/consumer name and pronunciation-
boundary groups, then review regression and EMA Lightning groups are appended.
The catalog now has 179 cases with the earlier 170 as an exact prefix. Python
fingerprints and manifests cover the catalog and every referenced file rather
than just the catalog itself.
Each short/medium policy cohort is separate, with 2,000 warmups and 10,000 calls.
Golden outcomes are checked before/after timing; class counts and exact inputs
are validated by the verifier. Native reports also retain 4/16/32 KiB grouping,
quotation/list, single malformed comma/dot tokens, mixed quantity lists,
name/phrase/prefix-miss/rejected-list and decomposed-Unicode
scaling diagnostics. These workloads are not exhaustive complexity checks.

The first process initialization is recorded separately from 100 later cached
same-process constructor calls. Those later calls are **not cold starts**.
Large 4/16/32 KiB inputs, maximum hints, candidate/result errors and long
identifiers are separate scaling diagnostics, not 1 ms guarantees.

## Python and measurement limits

Installed Python measurements include public argument validation, native Rust
work, marshaling and result/error disposal. Eight-thread public-call timings
are separate from the Rust target and from model/worker/audio costs.
Native allocation/peak instrumentation is unavailable; no inferred allocation
counts or peak-memory claims are made.

Record host/build context without changing power settings or stopping other
work. Shared-host load varies; GitHub-hosted runners and other hardware are not
assumed equivalent.

## Development measurement snapshot

This comparison used the captured feature implementation
[`a4e3055`](https://github.com/erdemtuna/normalizer-tr/commit/a4e3055) and the module
refactor subsequently committed as
[`0b3eb5d`](https://github.com/erdemtuna/normalizer-tr/commit/0b3eb5d).
Measurements were collected before the refactor commit, not from a clean release
revision. Both builds reported `normalizer-tr/0.4.0`; that identity alone does
not distinguish these source snapshots.

This historical snapshot predates the initialism and foreign-name additions;
it is not a measurement of the 0.5.0 release revision.

Host: AMD Ryzen AI 7 PRO 350, 8 cores / 16 logical processors, Windows 11
Enterprise 26200. Native builds used Rust 1.99.0 x64 MSVC, optimized bench
profile with `serde`; installed-wheel measurements used CPython 3.13.15.
Sampling used the frozen original Preserve/Reject cohorts and methodology above,
serially on the same shared host.

| Measurement / cohort | Baseline p95, microseconds | Refactored p95, microseconds |
|---|---|---|
| Native short, three repetitions | 27.4, 27.7, 22.9 | 27.1, 40.2, 20.8 |
| Native medium, three repetitions | 196.7, 245.3, 187.5 | 185.9, 210.1, 194.6 |
| Installed Python short, one repetition | 56.6 | 50.3 |
| Installed Python medium, one repetition | 232.3 | 204.2 |

All native repetitions met the host-specific target. Alternating comparison
runs retained class-level variability; these values do not establish a universal
speedup, identical code generation or a guarantee for amplified fallback output.
Frozen inputs and full benchmark outcomes were unchanged.

Raw reports and packages remain local development artifacts outside Git, not
published release evidence. A clean-revision full verification remains required
for release. Optimize measured bottlenecks or regressions; once the target is
met, prioritize clarity over additional performance machinery.
