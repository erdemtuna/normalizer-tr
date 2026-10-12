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

REPETITIONS = 3
SAMPLES = 10000
SCALING_SAMPLES = 100
QUANTILES = ("p50_ns", "p95_ns", "p99_ns", "max_ns")


def validate_timing(row, count=None):
    values = [row[key] for key in QUANTILES]
    if (
        any(type(value) is not int or value <= 0 for value in values)
        or values != sorted(values)
        or type(row["count"]) is not int
        or row["count"] <= 0
        or (count is not None and row["count"] != count)
    ):
        raise RuntimeError(f"invalid or undersampled timing: {row}")


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


def scaling_fingerprint(row):
    fingerprint = row.get("input_fingerprint")
    if fingerprint is None:
        return None
    if (
        not isinstance(fingerprint, dict)
        or set(fingerprint) != {"text", "options"}
        or not isinstance(fingerprint["text"], str)
        or len(fingerprint["text"].encode("utf-8")) != row["bytes"]
        or not isinstance(fingerprint["options"], dict)
        or not isinstance(fingerprint["options"].get("ambiguity_policy"), str)
        or fingerprint["options"]["ambiguity_policy"].lower() != row["policy"]
        or not isinstance(fingerprint["options"].get("hints"), list)
    ):
        raise RuntimeError(f"invalid recognition input fingerprint: {row['path']}")
    return hashlib.sha256(
        json.dumps(
            fingerprint, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()


def index_scaling_rows(rows):
    indexed = {}
    for row in rows:
        key = (row["path"], row["bytes"], row["policy"])
        if key in indexed:
            raise RuntimeError(f"duplicate recognition scaling workload: {key}")
        validate_timing(row["timing"], SCALING_SAMPLES)
        indexed[key] = {**row, "input_sha256": scaling_fingerprint(row)}
    return indexed


def compare(base, head, same_inputs, contract_changes=()):
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
    policy_inputs_equal = (
        base["expanded_coverage_measurement"]["input_cases"]
        == head["expanded_coverage_measurement"]["input_cases"]
    ) and not any(row["id"].startswith(("base:", "head:")) for row in contract_changes)
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
            policy_inputs_equal,
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
    before = index_scaling_rows(base["reviewed_recognition_scaling"]["scaling"])
    after = index_scaling_rows(head["reviewed_recognition_scaling"]["scaling"])
    if not before or before.keys() != after.keys():
        raise RuntimeError("recognition scaling workloads differ")
    result["recognition_scaling"] = [
        {
            "path": key[0],
            "bytes": key[1],
            "policy": key[2],
            "base_input_sha256": before[key]["input_sha256"],
            "head_input_sha256": after[key]["input_sha256"],
            **timing_rows(
                {str(key): before[key]["timing"]},
                {str(key): after[key]["timing"]},
                before[key]["input_sha256"] is not None
                and before[key]["input_sha256"] == after[key]["input_sha256"]
                and semantic_outcome(before[key]["outcome"])
                == semantic_outcome(after[key]["outcome"]),
            )[0],
        }
        for key in before
    ]
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--head", default=ROOT, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    base, head, output = args.base.resolve(), args.head.resolve(), args.output.resolve()
    revisions = source_revisions(base, head, ROOT)
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
            reports[name] = json.loads(path.read_text(encoding="utf-8"))
        paired.append(
            compare(
                reports["base"],
                reports["head"],
                corpus_hashes["base"] == corpus_hashes["head"],
                contract_changes,
            )
        )
    comparison = {
        "base_sha": revisions[base],
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
        "\nFull distributions, separate policies, and reviewed scaling cases: `comparison.json` and raw reports.\n"
    )
    summary = "\n".join(lines)
    (output / "summary.md").write_text(summary, encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
            stream.write(summary)
    print(summary)


if __name__ == "__main__":
    main()
