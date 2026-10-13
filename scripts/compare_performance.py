"""Paired native measurements, with semantic comparability instead of a timing gate."""

import argparse
import hashlib
import json
import os
import platform
import subprocess
from pathlib import Path

from .common import ROOT, check_source_revisions, source_revisions, write
from .compare_contracts import run_comparison, semantic_outcome
from .performance_report import (
    VERSION,
    admit_native,
    inventory,
    load,
    validate_timing,
)

REPETITIONS = 3
SAMPLES = 10000
SCALING_SAMPLES = 100
QUANTILES = ("p50_ns", "p95_ns", "p99_ns", "max_ns")
SUMMARY_ROWS_PER_SUITE = 5
PRE_V2_BASE_SHA = "e82bb205c091ca65d97ea8d24a7dcdb6d4fb511b"
V2_REFERENCE_SHA = "233c099cf25599ee13193d10e0a2260851f3eced"


def select_reference(pr_base, requested_base):
    reference = pr_base or requested_base
    if not reference or any(char.isspace() or ord(char) < 32 for char in reference):
        raise RuntimeError("performance comparison requires a nonempty exact base ref")
    if pr_base == PRE_V2_BASE_SHA:
        return V2_REFERENCE_SHA
    return reference


def comparison_basis(requested_sha, measured_sha):
    if requested_sha == measured_sha:
        kind = "requested-base"
        explanation = "Measurements use the requested base revision."
    elif (requested_sha, measured_sha) == (PRE_V2_BASE_SHA, V2_REFERENCE_SHA):
        kind = "v2-instrumented-reference"
        explanation = (
            "The requested PR base predates version-two instrumentation. "
            "Measurements use the frozen instrumented reference, not the literal PR base."
        )
    else:
        raise RuntimeError("unapproved performance reference substitution")
    return {
        "kind": kind,
        "requested_base_sha": requested_sha,
        "measurement_base_sha": measured_sha,
        "explanation": explanation,
    }


def snapshots_equal(base, head):
    return {key: semantic_outcome(value) for key, value in base.items()} == {
        key: semantic_outcome(value) for key, value in head.items()
    }


def timing_rows(base, head, comparable):
    if base.keys() != head.keys():
        raise RuntimeError("measurement cohorts differ")
    rows = []
    for key in base:
        validate_timing(base[key])
        validate_timing(head[key])
        if base[key]["count"] != head[key]["count"]:
            raise RuntimeError(f"sample counts differ: {key}")
        rows.append(
            {
                "cohort": key,
                "comparable": comparable,
                "base": base[key],
                "head": head[key],
                "p95_ratio": head[key]["p95_ns"] / base[key]["p95_ns"]
                if comparable
                else None,
            }
        )
    return rows


def outcome_summary(outcome):
    if "result" in outcome:
        return {"kind": "result", "complete": outcome["result"]["complete"]}
    summary = {"kind": "error", "error": outcome["error"]}
    if "limit" in outcome:
        summary["limit"] = outcome["limit"]
    return summary


def outcome_label(summary):
    if summary["kind"] == "result":
        return "result (complete)" if summary["complete"] else "result (incomplete)"
    label = f"error: {summary['error']}"
    return f"{label} ({summary['limit']})" if "limit" in summary else label


def compare(base, head, same_inputs, contract_changes=()):
    before_report, after_report = admit_native(base), admit_native(head)
    if base["method"] != head["method"]:
        raise RuntimeError("sampler methods differ")
    for report in (base, head):
        if set(report["cohorts"]) != {"short", "medium"}:
            raise RuntimeError("missing short or medium cohort")
        for section in (
            report["cohorts"],
            report["fallback_measurement"]["cohorts"],
            report["expanded_coverage_measurement"]["cohorts"],
        ):
            for row in section.values():
                validate_timing(row, SAMPLES)
        if set(report["fallback_measurement"]["cohorts"]) != {"short", "medium"}:
            raise RuntimeError("missing fallback cohorts")
        expected_policies = {
            f"{cohort}:{policy}"
            for cohort in ("short", "medium")
            for policy in ("preserve", "reject", "fallback")
        }
        if set(report["expanded_coverage_measurement"]["cohorts"]) != expected_policies:
            raise RuntimeError("missing policy contract cohorts")
    same_primary = (
        same_inputs
        and base["distribution"] == head["distribution"]
        and snapshots_equal(base["snapshots"], head["snapshots"])
    )
    same_fallback = same_inputs and snapshots_equal(
        base["fallback_measurement"]["snapshots"],
        head["fallback_measurement"]["snapshots"],
    )
    same_policy = (
        (
            base["expanded_coverage_measurement"]["input_cases"]
            == head["expanded_coverage_measurement"]["input_cases"]
        )
        and (
            base["expanded_coverage_measurement"]["inputs"]
            == head["expanded_coverage_measurement"]["inputs"]
        )
        and snapshots_equal(
            base["expanded_coverage_measurement"]["snapshots"],
            head["expanded_coverage_measurement"]["snapshots"],
        )
        and not any(
            row["id"].startswith(("base:", "head:")) for row in contract_changes
        )
    )
    result = {
        "primary": timing_rows(base["cohorts"], head["cohorts"], same_primary),
        "fallback": timing_rows(
            base["fallback_measurement"]["cohorts"],
            head["fallback_measurement"]["cohorts"],
            same_fallback,
        ),
        "policy_contract": timing_rows(
            base["expanded_coverage_measurement"]["cohorts"],
            head["expanded_coverage_measurement"]["cohorts"],
            same_policy,
        ),
        "per_class_policy": {
            "shared": timing_rows(
                {
                    key: value
                    for key, value in base["per_class_policy"].items()
                    if key in head["per_class_policy"]
                },
                {
                    key: head["per_class_policy"][key]
                    for key in base["per_class_policy"]
                    if key in head["per_class_policy"]
                },
                same_primary,
            ),
            "base_only": sorted(
                base["per_class_policy"].keys() - head["per_class_policy"].keys()
            ),
            "head_only": sorted(
                head["per_class_policy"].keys() - base["per_class_policy"].keys()
            ),
        },
    }
    before = {row.key: row for row in before_report.scaling}
    after = {row.key: row for row in after_report.scaling}
    if tuple(before) != tuple(after):
        raise RuntimeError("long workload order differs")
    result["scaling"] = []
    for key, row in before.items():
        other = after[key]
        reasons = []
        if row.input_sha256 != other.input_sha256:
            reasons.append("input/options changed")
        if semantic_outcome(row.outcome) != semantic_outcome(other.outcome):
            reasons.append("semantic outcome changed")
        result["scaling"].append(
            {
                "suite": key[0],
                "id": key[1],
                "bytes": key[2],
                "policy": key[3],
                "base_input_sha256": row.input_sha256,
                "head_input_sha256": other.input_sha256,
                "base_outcome": outcome_summary(row.outcome),
                "head_outcome": outcome_summary(other.outcome),
                "reason": "; ".join(reasons) or None,
                **timing_rows(
                    {str(key): row.timing}, {str(key): other.timing}, not reasons
                )[0],
            }
        )
    return result


def build_sampler(root, target):
    completed = subprocess.run(
        [
            "cargo",
            "bench",
            "--locked",
            "--features",
            "serde",
            "--bench",
            "latency",
            "--no-run",
            "--message-format=json",
        ],
        cwd=root,
        env={**os.environ, "CARGO_TARGET_DIR": str(target)},
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        check=True,
    )
    binaries = []
    for line in completed.stdout.splitlines():
        row = json.loads(line)
        if (
            row["reason"] == "compiler-artifact"
            and row["target"]["name"] == "latency"
            and row.get("executable")
        ):
            binaries.append(row["executable"])
    if len(binaries) != 1:
        raise RuntimeError(f"expected one latency executable, got {binaries}")
    return binaries[0]


def corpus_hash(root):
    return hashlib.sha256(
        b"".join(
            (root / "benches" / name).read_bytes()
            for name in ("corpus.json", "intent-corpus.json")
        )
    ).hexdigest()


def preflight(base, head):
    for root in (base, head, ROOT):
        if not (root / "benches" / "latency" / "workload_inventory.json").is_file():
            raise RuntimeError(
                f"{root}: missing version-two workload_inventory.json; use a clean "
                "common instrumented reference and regenerate native/Python reports"
            )
    catalogs = [inventory(root)[0] for root in (base, head, ROOT)]
    if any(value != catalogs[0] for value in catalogs[1:]):
        raise RuntimeError(
            "workload catalogs differ; use a clean common instrumented reference"
        )
    return catalogs[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--head", default=ROOT, type=Path)
    parser.add_argument("--requested-base", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    base, head, output = args.base.resolve(), args.head.resolve(), args.output.resolve()
    roots = (base, head, ROOT)
    requested_base = base
    if args.requested_base is not None:
        requested_base = args.requested_base.resolve()
        roots += (requested_base,)
    revisions = source_revisions(*roots)
    basis = comparison_basis(revisions[requested_base], revisions[base])
    catalog = preflight(base, head)
    corpus_hashes = {"base": corpus_hash(base), "head": corpus_hash(head)}
    output.mkdir()
    contract_changes = run_comparison(base, head, output / "contracts")
    executables = {
        name: build_sampler(root, output / f"{name}-target")
        for name, root in (("base", base), ("head", head))
    }
    paired = []
    for repetition in range(REPETITIONS):
        reports = {}
        order = ("base", "head") if repetition % 2 == 0 else ("head", "base")
        for name in order:
            path = output / f"{name}-{repetition + 1}.json"
            subprocess.run([executables[name], "--output", str(path)], check=True)
            reports[name] = load(path)
            if (
                reports[name].get("source_sha")
                != revisions[base if name == "base" else head]
            ):
                raise RuntimeError(f"{name} sampler source identity differs")
        paired.append(
            compare(
                reports["base"],
                reports["head"],
                corpus_hashes["base"] == corpus_hashes["head"],
                contract_changes,
            )
        )
    comparison = {
        "schema_version": VERSION,
        "report_kind": "native-comparison",
        "workload_inventory": catalog,
        "base_sha": revisions[base],
        "comparison_basis": basis,
        "head_sha": revisions[head],
        "tooling_sha": revisions[ROOT],
        "compiler": subprocess.check_output(["rustc", "-vV"], text=True),
        "host": platform.platform(),
        "runner": os.environ.get("RUNNER_NAME"),
        "corpus_sha256": corpus_hashes,
        "repetitions": paired,
        "contract_changes": contract_changes,
    }
    check_source_revisions(revisions)
    write(output / "comparison.json", comparison)
    lines = [
        "## Native performance comparison",
        "",
        f"Requested base: `{basis['requested_base_sha']}`; measured base: `{basis['measurement_base_sha']}`; head: `{revisions[head]}`.",
        "",
        basis["explanation"],
        "",
        "Informational only. Ratios compare equivalent inputs/outcomes; changed work is not a speedup.",
        "",
        "| Repetition | Primary cohort | Base p95 (us) | Head p95 (us) | Ratio |",
        "|---|---|---|---|---|",
    ]
    for index, pair in enumerate(paired, 1):
        for row in pair["primary"]:
            ratio = f"{row['p95_ratio']:.2f}" if row["comparable"] else "non-comparable"
            lines.append(
                f"| {index} | {row['cohort']} | {row['base']['p95_ns'] / 1000:.1f} "
                f"| {row['head']['p95_ns'] / 1000:.1f} | {ratio} |"
            )
    lines.append(
        "\nAll checked long-input comparisons (including work kinds and non-comparability reasons): `comparison.json`; complete input/options/outcomes: raw base/head reports. Comparable error timings measure failed work, not successful throughput.\n"
    )
    lines.extend(
        [
            "| Repetition | Suite | Compared rows | Non-comparable rows |",
            "|---|---|---:|---:|",
        ]
    )
    for index, pair in enumerate(paired, 1):
        suites = sorted({row["suite"] for row in pair["scaling"]})
        for suite in suites:
            selected = [row for row in pair["scaling"] if row["suite"] == suite]
            lines.append(
                f"| {index} | {suite} | {len(selected)} | {sum(not row['comparable'] for row in selected)} |"
            )
    lines.extend(
        [
            "",
            f"Largest {SUMMARY_ROWS_PER_SUITE} comparable p95 ratios per suite/repetition below (informational, not adoption thresholds). All rows and non-comparability reasons remain in `comparison.json`.",
            "",
            "| Repetition | Suite / workload | Bytes | Policy | Base work | Head work | Base p95 (us) | Head p95 (us) | Ratio |",
            "|---|---|---:|---|---|---|---:|---:|---:|",
        ]
    )
    for index, pair in enumerate(paired, 1):
        for suite in sorted({row["suite"] for row in pair["scaling"]}):
            selected = sorted(
                (
                    row
                    for row in pair["scaling"]
                    if row["suite"] == suite and row["comparable"]
                ),
                key=lambda row: row["p95_ratio"],
                reverse=True,
            )[:SUMMARY_ROWS_PER_SUITE]
            for row in selected:
                lines.append(
                    f"| {index} | {suite} / {row['id']} | {row['bytes']} | {row['policy']} "
                    f"| {outcome_label(row['base_outcome'])} | {outcome_label(row['head_outcome'])} "
                    f"| {row['base']['p95_ns'] / 1000:.1f} | {row['head']['p95_ns'] / 1000:.1f} | {row['p95_ratio']:.2f} |"
                )
    summary = "\n".join(lines)
    (output / "summary.md").write_text(summary, encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
            stream.write(summary)
    print(summary)


if __name__ == "__main__":
    main()
