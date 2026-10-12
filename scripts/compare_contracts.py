"""Compare public outcomes; golden tests, not this report, approve behavior."""

import argparse
import json
import os
import shutil
import subprocess
import tomllib
from pathlib import Path

from .common import (
    ROOT,
    check_source_revisions,
    policy_contract_fixture,
    source_revisions,
    write,
)

POLICIES = {"preserve", "reject", "fallback"}


def semantic_outcome(outcome):
    value = json.loads(json.dumps(outcome))
    if "result" in value:
        value["result"].pop("normalizer_id", None)
    return value


def index_rows(rows, case_ids=None):
    indexed = {}
    for row in rows:
        key = (row["id"], row["policy"])
        if key in indexed or key[1] not in POLICIES:
            raise RuntimeError(f"duplicate or invalid probe row: {key}")
        indexed[key] = semantic_outcome(row["outcome"])
    if not indexed:
        raise RuntimeError("probe produced no cases")
    expected_ids = {key[0] for key in indexed} if case_ids is None else set(case_ids)
    expected = {(case_id, policy) for case_id in expected_ids for policy in POLICIES}
    if indexed.keys() != expected:
        raise RuntimeError(
            f"probe case/policy coverage differs: missing={sorted(expected - indexed.keys())}, "
            f"unexpected={sorted(indexed.keys() - expected)}"
        )
    return indexed


def compare(base, head, case_ids=None):
    case_ids = None if case_ids is None else set(case_ids)
    before, after = index_rows(base, case_ids), index_rows(head, case_ids)
    if before.keys() != after.keys():
        raise RuntimeError("base and head probes did not run the same cases")
    return [
        {"id": key[0], "policy": key[1], "before": before[key], "after": after[key]}
        for key in before
        if before[key] != after[key]
    ]


def inputs(base, head):
    cases, seen = [], set()
    for origin, root in (("base", base), ("head", head)):
        for case in policy_contract_fixture(root)[0]:
            payload = {key: case[key] for key in ("id", "text", "hint") if key in case}
            fingerprint = json.dumps(payload, sort_keys=True)
            if fingerprint not in seen:
                seen.add(fingerprint)
                cases.append({**payload, "id": f"{origin}:{case['id']}"})
    cases.extend(
        [
            {"id": "error:empty", "text": ""},
            {"id": "error:bidi", "text": "a\u202e12"},
            {
                "id": "error:component-hint",
                "text": "2 x 3",
                "hint": {"start": 0, "end": 1, "kind": "cardinal"},
            },
        ]
    )
    return cases


def probe(root, directory, cases):
    directory.mkdir()
    manifest = directory / "Cargo.toml"
    dependency = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))[
        "dev-dependencies"
    ]["serde_json"]
    manifest.write_text(
        "[workspace]\n"
        '[package]\nname="contract-probe"\nversion="0.0.0"\nedition="2024"\n'
        '[[bin]]\nname="contract-probe"\n'
        f"path={json.dumps((ROOT / 'scripts' / 'rust' / 'contract_probe.rs').as_posix())}\n"
        "[dependencies]\n"
        f"normalizer-tr={{path={json.dumps(root.as_posix())},features=['serde']}}\n"
        f"serde_json={json.dumps(dependency)}\n",
        encoding="utf-8",
    )
    env = {**os.environ, "CARGO_TARGET_DIR": str(directory / "target")}
    shutil.copyfile(root / "Cargo.lock", directory / "Cargo.lock")
    subprocess.run(
        ["cargo", "build", "--release", "--manifest-path", str(manifest)],
        env=env,
        check=True,
    )
    subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            str(manifest),
        ],
        env=env,
        stdout=subprocess.DEVNULL,
        check=True,
    )
    executable = (
        directory
        / "target"
        / "release"
        / ("contract-probe.exe" if os.name == "nt" else "contract-probe")
    )
    completed = subprocess.run(
        [str(executable)],
        input=json.dumps(cases),
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        check=True,
    )
    rows = json.loads(completed.stdout)
    index_rows(rows, {case["id"] for case in cases})
    write(directory / "outcomes.json", rows)
    return rows


def run_comparison(base, head, output):
    revisions = source_revisions(base, head, ROOT)
    output.mkdir()
    cases = inputs(base, head)
    write(output / "inputs.json", cases)
    before = probe(base, output / "base", cases)
    after = probe(head, output / "head", cases)
    changes = compare(before, after, {case["id"] for case in cases})
    check_source_revisions(revisions)
    write(
        output / "comparison.json",
        {
            "base_sha": revisions[base],
            "head_sha": revisions[head],
            "tooling_sha": revisions[ROOT],
            "cases": len(cases),
            "changes": changes,
        },
    )
    summary = (
        f"## Public behavior comparison\n\n{len(changes)} changed case/policy outcomes "
        f"across {len(cases)} inputs. Full before/after records: `comparison.json`.\n"
        "\nIntentional changes belong in reviewed golden fixtures and migration notes.\n"
    )
    if changes:
        summary += "\n| Case | Policy | Changed fields |\n|---|---|---|\n"
        for row in changes:
            before, after = row["before"], row["after"]
            fields = [
                key
                for key in sorted(before.keys() | after.keys())
                if before.get(key) != after.get(key)
            ]
            if "result" in fields and "result" in before and "result" in after:
                fields = [
                    f"result.{key}"
                    for key in sorted(before["result"].keys() | after["result"].keys())
                    if before["result"].get(key) != after["result"].get(key)
                ]
            summary += f"| {row['id']} | {row['policy']} | {', '.join(fields)} |\n"
    (output / "summary.md").write_text(summary, encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
            stream.write(summary)
    print(summary)
    return changes


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--head", type=Path, default=ROOT)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    run_comparison(args.base.resolve(), args.head.resolve(), args.output.resolve())


if __name__ == "__main__":
    main()
