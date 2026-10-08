# Performance and measurement

The current normalizer met warm release Rust **p95 < 1 ms**, separately for
short (1-256) and medium (257-1024) original UTF-8 byte cohorts on the inspected
host. This is a measured development target, not a universal SLA, a hosted CI
timing gate or an end-to-end speech-generation guarantee.

The public inputs are in `benches/corpus.json` and `benches/intent-corpus.json`.
Do not shrink the corpus, weaken validation or remove slow cases to claim
improvement. Current semantic outcomes are checked independently by the
characterization and class tests.

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

Version 0.4 adds a separate `fallback_measurement` using the same frozen
cohorts, 2,000 warmups and 10,000 individually timed calls per cohort.
The original Preserve/Reject aggregate/order/corpus are unchanged. Fallback
results are not semantically equivalent to preservation/rejection, so do not
mix their samples into that aggregate or label differences as an optimization.
Python reports follow the same separation, including fallback-only concurrent
calls. No new universal latency promise is made for amplified code-point output.

Current source adds `expanded_coverage_measurement` without changing the frozen
inputs, original aggregate/order or existing report fields. Reviewed cases come
from `tests/fixtures/expanded-coverage.json`, shared with Rust/Python tests.
Each short/medium policy cohort is separate, with 2,000 warmups and 10,000 calls.
Golden outcomes are checked before/after timing; class counts and exact inputs
are validated by the verifier. Native reports also retain 4/16/32 KiB grouping,
quotation/list, malformed-input and decomposed-Unicode scaling diagnostics.
Python reports hash the fixture; the final manifest includes its hash too.

The first process initialization is recorded separately from 100 later cached
same-process constructor calls. Those later calls are **not cold starts**.
Large 4/16/32 KiB inputs, maximum hints, candidate/result errors and long
identifiers are separate scaling diagnostics, not 1 ms guarantees.

## Python, host and limits

Installed Python measurements include public argument validation, native Rust
work, marshaling and result/error disposal. Eight-thread public-call timings
are separate from the Rust target and from model/worker/audio costs.
Native allocation/peak instrumentation is unavailable; no inferred allocation
counts or peak-memory claims are made.

Inspected host: AMD Ryzen AI 7 PRO 350, 8 cores / 16 logical processors,
Windows 11 Enterprise 26200, Rust 1.94 x64 MSVC. Shared-host power/load/noise
context is recorded without changing settings or stopping other work. Timings
on GitHub-hosted runners or other hardware are not assumed equivalent.

### Current-source development comparison

On the same host using pinned Rust 1.99.0 x64 MSVC, the unchanged
`d0bc1bc` baseline and current uncommitted implementation produced these warm
original Preserve/Reject p95 values, in microseconds:

| Cohort | Baseline repetitions | Current-source repetitions |
|---|---|---|
| Short | 23.6, 28.8, 26.1 | 22.6, 18.1, 21.5 |
| Medium | 155.1, 159.8, 239.1 | 171.1, 155.9, 154.6 |

All current repetitions met the existing host-specific 1 ms target. Additional
isolated-baseline comparisons showed substantial shared-host variability, so
these results are not a universal improvement claim. A no-single-quote fast
path avoids unnecessary quotation traversal; numeric-run indexing avoids
repeated grouping lookahead. Original corpora and their hashes were unchanged.

The final separate expanded-coverage repetition had native p95 values of
9.4/9.2/9.6 microseconds for short Preserve/Reject/Fallback and
100.3/94.5/144.4 for medium. Installed CPython 3.13 original-cohort p95 was
63.0/236.5 microseconds for short/medium, compared with baseline 53.7/223.1.
These retain host noise, marshaling and disposal rather than hiding them.
Large-input/result-limit diagnostics are not covered by the 1 ms target.

Reports/packages are local session artifacts, not published release evidence.
The clean-revision full verifier remains a separate prerequisite for a release;
this development comparison does not bypass it.

### Module-refactor comparison

The behavior-preserving module refactor was compared with the captured
feature implementation committed as `a4e3055`, not the earlier 0.4 source.
Three retained native repetitions had original-cohort p95 of
27.1/40.2/20.8 microseconds for short inputs and 185.9/210.1/194.6 for medium.
Baseline repetitions were 27.4/27.7/22.9 and 196.7/245.3/187.5 respectively.
All refactored repetitions met the existing host-specific target.

Additional alternating baseline/refactor runs retained class-level variability;
this is not an assertion of identical code generation or a universal speedup.
Installed Python p95 was 50.3/204.2 microseconds for short/medium versus
56.6/232.3 before. Frozen inputs, complete benchmark snapshots and 351 additional
serialized public outcomes were identical; budget-relevant record sizes were
unchanged. Reports remain development artifacts outside Git.

Retained mechanisms are borrowed already-NFC grapheme-safe mapping with an NFC
fallback, indexed overlap windows, exact numerals and one immutable pattern
cache. There is no input/result memoization. Optimize only a measured bottleneck
or regression; once the target passes, prioritize clarity rather than more
performance machinery. Historical local reports/packages are not public
repository contents or a selectable comparison mode.
