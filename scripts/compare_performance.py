"""Paired native measurements, with semantic comparability instead of a timing gate."""

import argparse
import hashlib
import json
import os
import platform
import subprocess
from pathlib import Path

from .common import ROOT, check_source_revisions, source_revisions, write
from .compare_contracts import run_comparison
from .performance_comparison import compare
from .performance_report import VERSION, inventory, load
from .performance_report import validate_timing as validate_timing
from .performance_summary import render_summary

REPETITIONS = 3
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
                base_root=base,
                head_root=head,
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
    summary = render_summary(comparison)
    (output / "summary.md").write_text(summary, encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
            stream.write(summary)
    print(summary)


if __name__ == "__main__":
    main()
