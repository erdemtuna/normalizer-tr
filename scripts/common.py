"""Shared repository paths, report IO, and ordered policy-fixture loading."""

import hashlib
import json
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def record(path):
    return {"path": str(path), "sha256": sha(path), "bytes": path.stat().st_size}


def write(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)


def version(root=None):
    return tomllib.loads(
        ((ROOT if root is None else root) / "Cargo.toml").read_text(encoding="utf-8")
    )["package"]["version"]


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def source_revisions(*roots):
    revisions = {}
    for root in dict.fromkeys(roots):
        args = ["git", "-C", str(root)]
        revision = subprocess.check_output(
            [*args, "rev-parse", "HEAD"], text=True
        ).strip()
        changes = subprocess.check_output(
            [*args, "status", "--porcelain=v1", "--untracked-files=no"], text=True
        ).strip()
        if changes:
            raise RuntimeError(
                f"comparison requires clean tracked sources: {root}\n{changes}"
            )
        if (
            subprocess.check_output([*args, "rev-parse", "HEAD"], text=True).strip()
            != revision
        ):
            raise RuntimeError(
                f"source revision changed while capturing provenance: {root}"
            )
        revisions[root] = revision
    return revisions


def check_source_revisions(expected):
    actual = source_revisions(*expected)
    for root, revision in expected.items():
        if actual[root] != revision:
            raise RuntimeError(f"source revision changed during comparison: {root}")


def policy_contract_fixture(root=None):
    catalog = (
        (ROOT if root is None else root) / "tests" / "fixtures" / "policy-contract.json"
    )
    files = [catalog] + [
        catalog.parent / name
        for name in json.loads(catalog.read_text(encoding="utf-8"))
    ]
    cases = [
        case
        for file in files[1:]
        for case in json.loads(file.read_text(encoding="utf-8"))
    ]
    if not cases or len({case["id"] for case in cases}) != len(cases):
        raise RuntimeError("policy-contract fixture must have unique nonempty cases")
    return cases, files
