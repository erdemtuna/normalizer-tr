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

### PR evidence

The non-required **Performance evidence** workflow compares exact base/head
release builds serially on the same runner, with three paired repetitions and
alternating order. It retains this sampler's inputs, warmups, call disposal,
outliers and separate policy/scaling sections. The Actions summary links raw
reports and checked comparisons; slower timings alone do not fail a PR.

Input fingerprints and outcomes must agree before a timing ratio is interpreted
as equivalent work. Public policy outcomes are checked with the same probe
against both cores; changed work is labelled non-comparable, not an optimization.
Build/tool/report failures remain failures. No hosted one-millisecond gate or
new heap-allocation claim is made.

Both native and installed-Python raw reports now require **schema version 2**
and an explicit report kind. Version-one, missing-version and unsupported
reports are rejected with regeneration guidance, not read through a legacy
adapter. The library API is unchanged. Use a clean instrumentation-only
reference containing the same checked workload inventory as head before
measuring a candidate. The initial migration against an old reference fails
actionably in the non-required performance workflow; it is not a green skip.

`benches/latency/workload_inventory.json` is ordered coverage/method metadata,
not another input or golden catalog. Native reports retain all 316 original
long-input rows plus 123 scanner, 132 source-map, 48 Unicode fallback and six
Input/Hints over-limit rows. The resulting 625 rows are declared coverage,
not a sampling quota or an excuse to drop unfavorable work. Module generators
and independent checks remain in Rust; the comparator and verifier reject
duplicates, missing rows (including shared omissions), changed inventories,
partial outcomes and malformed timings. Exact text/options and full outcomes
are captured outside timed calls. Only identical validated work receives
a ratio; changed valid inputs/outcomes retain both timings and an explicit
non-comparability reason. Genuine limit kinds remain recorded outcomes.
All 625 actual recipe keys are checked against that inventory before the first
clock or normalizer initialization, then the same prepared cases are measured
in the original order. The original recipes are shared in
`benches/latency/legacy_inputs.rs`; default-options construction and disposal
inside the original large-input timer remain unchanged. The final emitted-row
check remains as a second guard.

Three serial base/head pairs retain their alternating order. The Markdown
summary lists coverage/non-comparability by long-input suite; detailed row
identities, timings, digests and reasons are in `comparison.json`, and full
text/options/outcomes remain in the raw reports. Deadline observations remain
separate, without ratios or a hard wall-clock assertion.
The summary also shows the five largest comparable p95 ratios per suite and
repetition; that display bound is not an adoption threshold or evidence of
repeatable gains.

The informational workflow has a **90-minute execution ceiling**, an operational
policy rather than a hosted runtime estimate or latency SLA. A full same-source
Windows reference smoke at `233c099` took 2,306,680.9043 ms (about 38.4 minutes),
exceeding the previous 30-minute ceiling while retaining all three serial pairs,
625 rows, samples and outliers. Its six native raw reports totaled 994,166,187
uncompressed bytes; the installed-Python artifact is separate. Artifact upload
uses explicit deflate compression level 6, preserving the existing default.
An actual local level-6 ZIP of those six unchanged raw reports was 24,721,010
bytes and took about 10.54 seconds to produce; all entries passed CRC validation.
That compressed-size observation is not hosted upload duration or peak memory.
Hosted upload duration, separate serializer CPU/write costs and peak/allocator
measurements were not available. Filesystem timestamp intervals are not those
measurements.

Identical-source p95 ratios in that shared-host smoke ranged from approximately
0.336 to 9.413, including a 5.15 failed-phone row. These are baseline variability
observations, not gains or regressions. Later adoption needs repeatable full-call
evidence beyond observed variation and no meaningful counter-path regression;
do not remove noisy or slow cases to improve an aggregate.

### Bounded optimization investigations

Four isolated candidates were compared against the clean instrumented
`233c099` reference, using `c9e4b85` comparison tooling. Each retained three
fresh alternating base/head pairs, all 625 long workloads, full-call disposal,
all outliers and the original frozen cohorts. All 7,500 long-input pairs had
equal original inputs/options and full semantic outcomes; each public contract
comparison reported zero changes across 237 inputs.

Independent evidence review deferred all four production changes:

| Investigation | Candidate | Decision and material evidence |
|---|---|---|
| #25, whole-input list gate | `2327de2` | Defer. Comma-free gains overlapped matched same-source variation; positive list and other counter-path costs were not cleared. |
| #26, address possibility gate | `69d819f` | Defer. Simple URL-negative gains were persuasive, but inferred/positive address counter-path warnings prevented a no-regression conclusion. |
| #28, ASCII source mapping | `92a4728` | Defer. Borrowed ASCII really eliminated per-input boundary storage, but repeated small-Unicode median and p95 warnings remained despite that permitted structural benefit. |
| #29, direct code-point formatting | `ed57bf7` | Defer. Successful scalar-fallback p95 improved narrowly, but family medians and named/Preserve/Reject controls did not establish the required full-call benefit and counter-path clearance. |

These are evidence-backed non-adoption decisions, not proofs that every
measured slowdown was caused by the candidate. In particular, unusually fast
baseline repetitions account for some large ratios. Review matched workload,
size and policy distributions, medians and tails rather than dismissing all
counterevidence using the global noise extrema. Faster genuine Result errors
are not successful-output throughput gains. The source-map storage exception
did not waive behavior or regression checks, and storage representation was
not used to infer allocator-event counts or peak memory.

Production recognition, source mapping and rendering remain unchanged.
The instrumentation, characterization tests, candidate commits and complete
reports are retained for future investigation. The checked reports and
per-family p50/p95 evidence are local development artifacts, not published
release evidence or hosted runtime guarantees. No candidate inputs, policies,
samples or unfavorable rows were removed to obtain these decisions.

Both comparison CLIs require clean tracked sources and check source/revision
drift before emitting a successful comparison. Reports capture base, head and
probe-tooling revisions before building. Unrelated untracked files and ignored
build outputs are allowed; use an unchanged checkout for the entire run.

Run locally against a separate baseline checkout and new output directory:

```powershell
python -m scripts.compare_performance --base <baseline-checkout> --output <new-output-directory>
```

Keep the host-specific target below for local verification. Hosted comparisons
provide regression evidence, not a substitute for reviewing semantics or a
universal latency promise.

### Full local verification

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
It contains 234 cases in ordered fixture groups. Existing IDs, order and
unrelated expected outcomes are preserved; new nominal cases are appended.
The two Instagram/WhatsApp output expectations deliberately change as described
in the [0.6.0 migration notes](CHANGELOG.md#060---2026-10-11). Python fingerprints
and manifests cover the catalog and every referenced file, not just the catalog
itself.
Each short/medium policy cohort is separate, with 2,000 warmups and 10,000 calls.
Golden outcomes are checked before/after timing; class counts and exact inputs
are validated by the verifier. Native reports also retain 4/16/32 KiB grouping,
quotation/list, single malformed comma/dot tokens, mixed quantity lists,
name/phrase/prefix-miss/rejected-list and decomposed-Unicode
scaling diagnostics. These workloads are not exhaustive complexity checks.

The 0.6.0 scaling diagnostics add possessive chains, plural combinations,
suffixes with multiple grammatical meanings, phrases, wrong vowel harmony and
single long name tails. Name suffix matching is bounded and does not render
rejected candidate outputs. Catalog consistency is checked once during
initialization; include that cost in the first constructor measurement, not
cached constructor or warm call timings. Differences between workloads that
produce different normalized text do not establish a speedup.

The first process initialization is recorded separately from 100 later cached
same-process constructor calls. Those later calls are **not cold starts**.
Large 4/16/32 KiB inputs, maximum hints, candidate/result errors and long
identifiers are separate scaling diagnostics, not 1 ms guarantees.

The 0.7.0 scaling cases cover numeric/copular tails, quantity possession,
written-order rates, clocks/date tails, account periods, contextual address
fields and long flat products. Source ownership and control checks are part of
those workloads. Composite outputs charge each constituent, so reaching a limit
with a long chain is a real measured outcome, not an omitted sample.
Rate wording deliberately changes existing corpus outcomes without changing
the corpus inputs; timings are not evidence of equivalent semantic work.

`reviewed_recognition_scaling` is a separate report section for exact
1/4/8/32 KiB failed-telephone inputs and 4/8/16/32 KiB electronic trailing
parentheses. Each policy retains its text/diagnostic or genuine limit outcome
and 100 individually timed calls, plus three 10 ms electronic deadline probes.
Scaling rows also retain exact input/options snapshots outside the timed loop;
the comparison hashes these snapshots and requires equal identities and outcomes.
Older scaling reports must be regenerated; missing snapshots are a format error.
These address measured pre-existing quadratic paths, not regressions introduced
by the new notation grammar. Indexed phone traversal and linear parenthesis
trimming retain admission and ownership; the deadline probes record completion
or cancellation without a flaky wall-clock assertion. Frozen cohorts and the
existing 279 policy scaling cases are unchanged.

## Python and measurement limits

Installed Python measurements include public argument validation, native Rust
work, marshaling and result/error disposal. Eight-thread public-call timings
are separate from the Rust target and from model/worker/audio costs.
The sequential sections retain 2,000 warmups and 10,000 calls per cohort,
separate Preserve/Reject and Fallback distributions, and the six policy-contract
cohorts. The eight-thread sections retain 1,000 calls per cohort and measure
public invocation/GIL wait, not executor queueing, equality checks or result
disposal. Do not mix those worker samples with disposal-inclusive sequential
samples. Constructor observations in each section concern that invocation;
a Fallback-section first constructor is not the first core initialization
in the process.

The installed-Python version-two report retains exact prepared text/options,
order and complete facade result/error snapshots, including original byte
ranges, error messages/limit kinds and the derived `fallback_used` field.
Full snapshot capture and serialization remain outside timed calls. Native
Serde labels/ranges and Python facade labels/ranges remain distinct; neither
parser reimplements language or grapheme rules. Python coverage/counts are
checked against the frozen corpus and ordered policy fixtures, not expanded
to the native long-workload matrix. There is no Python one-millisecond gate
or cross-language timing ratio.

Run the installed sampler from the repository root with the same designated
Python environment used to build/reinstall and test the wheel:

```powershell
<absolute-installed-python> -m benches.python_latency --output <new-absolute-python-report.json>
```

It requires clean committed source and records source/tooling revision,
interpreter, facade/native paths, build/normalizer identity and loaded-native,
corpus and policy-fixture hashes. Final verification matches the report to
the authoritative installed consumer/wheel; a same-version stale module is
not acceptable. The full packaging verifier retains its existing fresh
external consumer role and uses that same consumer interpreter for installed
tests, consumer checks and Python sampling.

Regenerate both formats into new paths after an approved reference/catalog
change; never rewrite historical reports or combine evidence across inventories.
Inspect artifact sizes, serialization/write cost and whole serial workflow
duration under an exclusive measurement slot before changing timeout or scope.
No such cost or speedup is established merely by adding instrumentation.
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
