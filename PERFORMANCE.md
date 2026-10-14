# Performance and measurement

Use these measurements to compare equivalent normalization work and identify
scaling costs, not to promise speech-generation latency. The development target
is warm release Rust **p95 < 1 ms** for short (1-256) and medium (257-1024)
original UTF-8 byte cohorts. It is host-specific, not a universal SLA or hosted
CI timing gate. Long inputs, fallback amplification and cooperative deadlines
have separate diagnostics.

## Run a comparison

First provision the [development environment](CONTRIBUTING.md#set-up).
Reference, candidate and comparison-tooling checkouts must be committed, unchanged
and contain the same `benches/latency/workload_inventory.json`. Keep the host free
of competing builds or samplers; do not change power settings or stop other work.

For the current version-two inventory, this clean instrumentation reference is
available in the repository history. From the candidate checkout:

```powershell
$python = (Resolve-Path .\.venv\Scripts\python.exe).Path
$referenceSha = '233c099cf25599ee13193d10e0a2260851f3eced'
$reference = Join-Path (Split-Path (Get-Location).Path -Parent) 'normalizer-tr-reference'
$output = Join-Path $env:TEMP 'normalizer-tr-performance-01'
git worktree add --detach $reference $referenceSha
& $python -m scripts.compare_performance --base $reference --output $output
```

Use a new reference directory and output path; neither should already exist.
For another approved inventory, choose a clean instrumentation-only reference
containing that same catalog. The candidate defaults to the current checkout;
`--head <candidate-checkout>` allows a separate candidate while retaining the
current checkout as tooling. Timings are always measured freshly, never reused
from historical reports.

If preflight reports a missing version-two inventory or changed catalog, stop
and choose compatible instrumented sources. Native and installed-Python reports
require **schema version 2** and an explicit kind. Regenerate old reports;
there is no legacy reader, repaired-report adapter or missing-evidence skip.
The public library API is unaffected by this development-artifact format change.

### Hosted PR evidence

The non-required **Performance evidence** workflow first checks whether native
measurement is relevant. On PR open/reopen it examines the full PR diff; on
subsequent pushes it compares the previous and current heads, not the accumulated
PR diff. Documentation, media, ordinary test-only and Python-facade or
installed-sampler changes skip the expensive native comparison. Rust sources/data,
native workloads/fixtures, dependencies/toolchain or measurement tooling changes
still run it. The small
scope job records the decision; unavailable or mismatched source evidence fails
rather than pretending the change is irrelevant.

When extracting measurement tooling, update the path selection in
`scripts/ci_checks.py` and test a latest-push diff containing only the new
module. An older entry point remaining in the full PR diff does not select
that later push.

Manual dispatch always requests measurement. Documentation-only updates do not
cancel a relevant comparison already running; a newer relevant update supersedes
an older measurement. Retained artifacts identify the revisions actually
measured, not a later documentation-only head.

When selected, the workflow checks out the exact PR head and measures three
fresh serial base/head pairs with alternating order.
Ordinary PRs use their actual base; manual dispatch uses the requested commit
or tag. Build, source, inventory and report failures fail the job. Slower valid
timings alone do not.

One explicit instrumentation transition needs a different measurement reference:
the pre-v2 PR base `e82bb205c091ca65d97ea8d24a7dcdb6d4fb511b` uses the frozen v2
reference `233c099cf25599ee13193d10e0a2260851f3eced`. That reviewed reference keeps
the production engine unchanged while adding the evidence needed to measure it.
This is **not a literal PR-base timing comparison**. `comparison_basis` records
both revisions and the reason in `comparison.json`, and the Actions summary
identifies them before presenting ratios. Any other substitution is rejected.
Required public-API and policy compatibility checks still use the actual PR base.
When main contains v2 instrumentation, new PRs use that actual base automatically.

The job has a **90-minute ceiling** and uploads retained evidence with compression
level 6. These are capacity settings, not runtime or upload guarantees. One local
Windows reference run took about 38 minutes and produced roughly 1 GB of raw
reports, compressing to about 25 MB. Allow space for raw evidence and both build
trees; other hosts and workloads can differ substantially.

## Interpret the evidence

Start with `summary.md`, then inspect `comparison.json` for every checked long
row, not only the five largest comparable p95 ratios shown per suite/repetition.
Raw base/head reports retain exact text/options and complete outcomes.

The ratio is **head p95 / base p95**: `1.10` means 10% slower and `0.90` means 10%
faster for that comparable row. Read absolute times and p50 as well as tails.
Three repetitions help expose variation; a single ratio is not an adoption rule.

| Work label | Meaning |
|---|---|
| `result (complete)` | The call returned fully resolved normalization work. |
| `result (incomplete)` | The call returned preserved unresolved spans and issues. |
| `error: LimitExceeded (Result)` or another error | The call failed; its timing is not successful-output throughput. |
| `non-comparable` | Inputs/options or full semantic outcomes changed; both timings remain, but no optimization ratio is assigned. |

Full outcome equality determines comparability; compact labels are descriptive,
not a weaker substitute. Genuine limit failures remain measured work. Deadlines
are separate observations without ordinary ratios or hard wall-clock assertions.
Each native report is validated against its own source checkout's policy
fixtures. Appending reviewed fixtures does not invalidate the older baseline
report, but changed policy cohorts remain non-comparable.
Shared-host noise can be large: review matched workload, size and policy medians
and tails rather than using global extremes to dismiss counterevidence.

Adopt an optimization only with repeatable full-call benefit beyond observed
variation and no meaningful counter-path regression. Verified simpler mapping
storage can justify consideration without a latency gain, but does not waive
behavior fidelity or regression checks. Text tests are not audio validation.

## Native methodology

- `Instant` measures 10,000 individual calls per cohort after 2,000 warmup calls.
- A deterministic mix interleaves Preserve/Reject policy cases; outcomes are
  checked before and after sampling.
- Timings include owned result/error disposal, not just recognition.
- Nearest-rank p50/p95/p99/max and per-class/policy counts are reported.
- Every outlier is retained. Clock overhead is reported, not subtracted.
- Three repetitions are retained by the full verifier; results are not replaced
  by a batched average or confidence interval.

The public cohorts live in `benches/corpus.json` and `benches/intent-corpus.json`.
Separate Fallback and policy-contract cohorts retain the same warmups and sample
counts; do not mix work across policies. Ordered policy fixtures live in
`tests/fixtures/policy-contract.json` and are shared with Rust/Python checks.

Long workloads have 100 individually timed full calls without added warmup.
The ordered inventory declares complete coverage across malformed numeric tokens,
products/addresses, identifiers, Unicode, hints and genuine limits. Preparation
checks the actual recipe keys before initialization or timing; emission checks
coverage again. Neither guard replaces independent semantic checks.

The first process initialization is separate from 100 subsequent same-process
constructor calls, which are **not cold starts**. Recognition diagnostics include
three 10 ms electronic deadline observations; cancellation is cooperative, not
a hard latency guarantee. See [reviewed boundaries](docs/normalization.md#reviewed-boundary-regressions-050).

Keep frozen inputs/order, default-options construction, disposal and outliers.
Do not shrink corpora, regenerate unrelated goldens or drop slow/error rows to
improve an aggregate. Exact snapshots, independent checks, fingerprints and
serialization stay outside call timers. The buffered JSON writer checks its
final flush and avoids an additional full serialized-string copy; it still
retains the full report tree and establishes no measured peak-memory claim.

To run only the native sampler, use a new report path:

```powershell
cargo bench --locked --features serde --bench latency -- --output <new-absolute-report.json>
```

## Python and measurement limits

Rebuild, reinstall and test the exact wheel with one designated environment
before sampling; version equality alone cannot detect a stale native module.
Use the [wheel build command](bindings/python/README.md#development-and-verification)
with the designated interpreter, install that fresh wheel with
`& $python -m pip install --force-reinstall --no-deps <wheel-path>`, then run its
installed tests from [CONTRIBUTING.md](CONTRIBUTING.md#checks). From clean committed
source, with that same interpreter:

```powershell
& $python -m benches.python_latency --output <new-absolute-python-report.json>
```

Sequential cohorts use 2,000 warmups and 10,000 calls, including public argument
validation, native work, marshaling and result/error disposal. Separate
eight-thread cohorts use 1,000 calls and measure invocation/GIL wait, not executor
queueing, equality checks or disposal. Do not combine these methods or assign
Rust/Python ratios or a Python one-millisecond gate. A later section's first
constructor is not first-process initialization.

Snapshots retain exact options, original byte ranges, error messages/limit
kinds and derived `fallback_used`. Native Serde and Python facade shapes remain
distinct; the parser does not implement grammar or grapheme rules. Python
coverage follows its frozen corpora and policy fixtures, not the native long
matrix. Reports record source/interpreter and loaded-native identity; full
verification matches the authoritative installed consumer and wheel.

## Full local verification

Follow [CONTRIBUTING.md](CONTRIBUTING.md#full-verification) to provision the
isolated tools and advisory database, then run from a clean Windows checkout:

```powershell
.\scripts\verify.ps1 -PythonPath <absolute-build-python> -OutputDirectory <new-absolute-directory>
```

Optional `-RustupHome` selects an existing isolated toolchain. Output must be new;
reports/packages are not overwritten. The serial workflow checks packages,
external consumers, source/tests/docs and measurements, then writes a checksum
manifest. It does not publish, run speech models or change global settings.

## Historical investigations

The experiments below were deferred: no production list/address gate, ASCII
mapping optimization or direct code-point formatter was adopted.

| Investigation | Why it was not adopted |
|---|---|
| [List scanning](https://github.com/erdemtuna/normalizer-tr/issues/25) | Target gains overlapped matched variation; positive-list counter-path costs were not cleared. |
| [Address scanning](https://github.com/erdemtuna/normalizer-tr/issues/26) | Simple URL-negative gains were persuasive, but real-address counter-path warnings remained. |
| [ASCII source mapping](https://github.com/erdemtuna/normalizer-tr/issues/28) | Boundary storage decreased, but repeated small-Unicode median/tail warnings remained. |
| [Unicode formatting](https://github.com/erdemtuna/normalizer-tr/issues/29) | Successful scalar-fallback tails improved narrowly; family medians and controls did not clear the adoption criterion. |

[PR #37](https://github.com/erdemtuna/normalizer-tr/pull/37) preserves the decisions
and source/method attribution. Experimental commits and full local raw ledgers
are not publicly fetchable release evidence. Non-adoption does not prove that
every slowdown was caused by a candidate; anomalously fast baselines can also
produce large ratios. Faster Result errors are not successful throughput.

The [archived 0.4 development snapshot](https://github.com/erdemtuna/normalizer-tr/blob/0d4a5d259400a0810f71fca8cd50344eac8da33c/PERFORMANCE.md#development-measurement-snapshot)
is historical, not a performance claim for the current release. Keep historical
reports unchanged. Native allocator/peak measurements, separate serializer/write
costs, hosted upload duration and audio validation are not established by these
records. Prioritize clarity once measured requirements are met.
