"""Small CI decisions shared with their failure-path tests."""

import argparse
import json
import os
import re
import subprocess

from .common import ROOT

DEPENDENCY_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "bindings/python/Cargo.toml",
    "bindings/python/pyproject.toml",
    "bindings/python/requirements-dev.txt",
    "bindings/python/constraints.lock",
    ".github/dependabot.yml",
}


def dependency_changes(paths):
    return any(
        path in DEPENDENCY_FILES or path.startswith(".github/workflows/")
        for path in paths
    )


def require_success(jobs):
    if not jobs or any(job["result"] != "success" for job in jobs.values()):
        raise RuntimeError(f"required CI jobs did not all succeed: {jobs}")


def check_tool_pins(root=ROOT):
    def pins(path):
        return {
            name.lower().replace("_", "-"): version
            for name, version in re.findall(
                r"^([A-Za-z0-9_.-]+)==([^\s]+)$",
                path.read_text(encoding="utf-8"),
                re.MULTILINE,
            )
        }

    requirements = pins(root / "bindings/python/requirements-dev.txt")
    constraints = pins(root / "bindings/python/constraints.lock")
    for name, version in requirements.items():
        if constraints.get(name) != version:
            raise RuntimeError(f"development constraint differs: {name}")
    for workflow in (root / ".github/workflows").glob("*.yml"):
        text = workflow.read_text(encoding="utf-8")
        for name, version in re.findall(r"\b([A-Za-z0-9_-]+)==([0-9.]+)", text):
            if name in requirements and requirements[name] != version:
                raise RuntimeError(f"{workflow.name} pins a different {name}")
        for version in re.findall(r'maturin-version:\s*["\']?([0-9.]+)', text):
            if requirements["maturin"] != version:
                raise RuntimeError(f"{workflow.name} pins a different maturin")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("gate", "dependencies", "pins"))
    parser.add_argument("--base")
    args = parser.parse_args()
    if args.action == "gate":
        require_success(json.loads(os.environ["CI_NEEDS"]))
    elif args.action == "pins":
        check_tool_pins()
    else:
        changed = True
        if args.base:
            paths = subprocess.check_output(
                ["git", "diff", "--name-only", args.base, "HEAD"], text=True
            ).splitlines()
            changed = dependency_changes(paths)
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
            stream.write(f"audit={str(changed).lower()}\n")


if __name__ == "__main__":
    main()
