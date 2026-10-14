"""Small CI decisions shared with their failure-path tests."""

import argparse
import json
import os
import re
import subprocess
import tomllib
from importlib import metadata
from pathlib import Path

from .common import ROOT

DEPENDENCY_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "bindings/python/Cargo.toml",
    "bindings/python/pyproject.toml",
    "bindings/python/requirements-dev.in",
    "bindings/python/requirements-dev.txt",
    "bindings/python/constraints.lock",
    ".github/dependabot.yml",
}
PORTABILITY_PACKAGES = {
    "colorama",
    "pywin32-ctypes",
    "secretstorage",
    "importlib-metadata",
    "backports-tarfile",
}
PERFORMANCE_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "benches/latency.rs",
    "benches/corpus.json",
    "benches/intent-corpus.json",
    "scripts/common.py",
    "scripts/ci_checks.py",
    "scripts/performance_comparison.py",
    "scripts/performance_report.py",
    "scripts/performance_summary.py",
    "scripts/verification.py",
    "scripts/verify.ps1",
    "scripts/rust/contract_probe.rs",
    ".github/workflows/performance.yml",
}
PERFORMANCE_PREFIXES = (
    "src/",
    "benches/latency/",
    "tests/fixtures/",
    "tests/support/",
)


def package_name(name):
    return re.sub(r"[-_.]+", "-", name).lower()


def read_requirements(path, *, source=False):
    requirements = {}
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.partition("#")[0].strip()
        if not line:
            continue
        match = re.fullmatch(
            r"([A-Za-z0-9][A-Za-z0-9_.-]*)(?:==([A-Za-z0-9][A-Za-z0-9.!+_-]*))?",
            line,
        )
        if not match:
            raise RuntimeError(f"{path}:{number}: expected an exact package pin")
        name = package_name(match[1])
        version = match[2]
        if name in requirements:
            raise RuntimeError(f"{path}:{number}: duplicate package: {name}")
        if version is None and not (source and name in PORTABILITY_PACKAGES):
            raise RuntimeError(f"{path}:{number}: missing exact pin: {name}")
        requirements[name] = version
    if not requirements:
        raise RuntimeError(f"empty tool requirements: {path}")
    return requirements


def tool_version(name, root=ROOT):
    source = read_requirements(
        root / "bindings/python/requirements-dev.in", source=True
    )
    name = package_name(name)
    if not source.get(name):
        raise RuntimeError(f"no direct tool pin: {name}")
    pins = read_requirements(root / "bindings/python/requirements-dev.txt")
    if pins.get(name) != source[name]:
        raise RuntimeError(f"development lock differs: {name}")
    return source[name]


def check_installed_tools(tools=None, root=ROOT):
    from packaging.requirements import Requirement
    from packaging.version import Version

    source = read_requirements(
        root / "bindings/python/requirements-dev.in", source=True
    )
    pins = read_requirements(root / "bindings/python/requirements-dev.txt")
    roots = list(source) if tools is None else [package_name(name) for name in tools]
    if not roots or any(name not in source for name in roots):
        raise RuntimeError(f"unknown or empty tool selection: {roots}")
    installed = {
        package_name(dist.metadata["Name"]): dist for dist in metadata.distributions()
    }
    pending = [(name, set()) for name in roots]
    visited = {}
    while pending:
        name, extras = pending.pop()
        if name in visited and extras <= visited[name]:
            continue
        extras = visited.get(name, set()) | extras
        visited[name] = extras
        if name not in pins or name not in installed:
            raise RuntimeError(f"tool dependency is not installed and pinned: {name}")
        dist = installed[name]
        if Version(dist.version) != Version(pins[name]):
            raise RuntimeError(f"installed tool dependency differs from lock: {name}")
        for raw in dist.requires or []:
            requirement = Requirement(raw)
            if requirement.marker and not any(
                requirement.marker.evaluate({"extra": extra}) for extra in extras | {""}
            ):
                continue
            dependency = package_name(requirement.name)
            if dependency not in installed or not requirement.specifier.contains(
                installed[dependency].version, prereleases=True
            ):
                raise RuntimeError(f"unsatisfied tool dependency: {name}: {raw}")
            pending.append((dependency, set(requirement.extras)))


def dependency_changes(paths):
    return any(
        path in DEPENDENCY_FILES or path.startswith(".github/workflows/")
        for path in paths
    )


def performance_changes(paths):
    return any(
        not path.endswith(".md")
        and (
            path in PERFORMANCE_FILES
            or path.startswith(PERFORMANCE_PREFIXES)
            or (path.startswith("scripts/compare_") and path.endswith(".py"))
        )
        for path in paths
    )


def performance_scope_base(event_name, action, before, pr_base):
    if event_name == "workflow_dispatch":
        return None
    if event_name != "pull_request" or action not in (
        "opened",
        "reopened",
        "synchronize",
    ):
        raise RuntimeError("unsupported performance-scope event")
    base = before if action == "synchronize" else pr_base
    if not isinstance(base, str) or re.fullmatch(r"[0-9a-f]{40}", base) is None:
        raise RuntimeError("performance scope requires the exact event base commit")
    return base


def select_performance_changes(base_checkout):
    event_name = os.environ["GITHUB_EVENT_NAME"]
    event = {}
    if event_name == "pull_request":
        event = json.loads(
            Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8")
        )
    base = performance_scope_base(
        event_name,
        event.get("action"),
        event.get("before"),
        event.get("pull_request", {}).get("base", {}).get("sha"),
    )
    if base is None:
        run = True
        reason = "Manual dispatch requests a complete measurement."
    else:
        if base_checkout is None:
            raise RuntimeError("performance scope requires the event-base checkout")
        base_checkout = base_checkout.resolve()
        actual = subprocess.check_output(
            ["git", "-C", str(base_checkout), "rev-parse", "HEAD"], text=True
        ).strip()
        if actual != base:
            raise RuntimeError("performance-scope checkout differs from event base")
        subprocess.run(
            [
                "git",
                "fetch",
                "--no-tags",
                "--update-shallow",
                str(base_checkout),
                "HEAD",
            ],
            cwd=ROOT,
            check=True,
        )
        paths = subprocess.check_output(
            ["git", "diff", "--no-renames", "--name-only", "-z", base, "HEAD", "--"],
            cwd=ROOT,
            text=True,
        ).split("\0")
        paths = [path for path in paths if path]
        run = performance_changes(paths)
        kind = "latest push" if event["action"] == "synchronize" else "full PR"
        reason = f"Checked {kind} changes from {base}: " + (
            "measurement inputs/code changed."
            if run
            else "no measurement inputs/code changed."
        )
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
        stream.write(f"run={str(run).lower()}\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
        stream.write(f"## Performance scope\n\n{reason}\n")


def require_success(jobs):
    if not jobs or any(job["result"] != "success" for job in jobs.values()):
        raise RuntimeError(f"required CI jobs did not all succeed: {jobs}")


def check_tool_pins(root=ROOT):
    if (root / "bindings/python/constraints.lock").exists():
        raise RuntimeError("retired constraints file remains")
    requirements = read_requirements(
        root / "bindings/python/requirements-dev.in", source=True
    )
    constraints = read_requirements(root / "bindings/python/requirements-dev.txt")
    for name, version in requirements.items():
        if name not in constraints or (
            version is not None and constraints[name] != version
        ):
            raise RuntimeError(f"development lock differs: {name}")
    build = tomllib.loads(
        (root / "bindings/python/pyproject.toml").read_text(encoding="utf-8")
    )["build-system"]["requires"]
    maturin = [
        requirement
        for requirement in build
        if package_name(requirement.partition("==")[0]) == "maturin"
    ]
    if maturin != [f"maturin=={requirements.get('maturin')}"]:
        raise RuntimeError("build-system pins a different maturin")
    for workflow in (root / ".github/workflows").glob("*.yml"):
        text = workflow.read_text(encoding="utf-8")
        for name, version in re.findall(r"\b([A-Za-z0-9_-]+)==([0-9.]+)", text):
            if package_name(name) in requirements:
                raise RuntimeError(f"{workflow.name} duplicates a tool pin: {name}")
        if re.search(r'maturin-version:\s*["\']?[0-9]', text):
            raise RuntimeError(f"{workflow.name} duplicates the maturin action pin")
        if "constraints.lock" in text:
            raise RuntimeError(f"{workflow.name} references the retired constraints")
    verification = root / "scripts/verify.ps1"
    if verification.exists():
        text = verification.read_text(encoding="utf-8")
        if "constraints.lock" in text or any(
            package_name(name) in requirements
            for name in re.findall(r"\b([A-Za-z0-9_-]+)==[0-9.]+", text)
        ):
            raise RuntimeError("verify.ps1 duplicates or uses retired tool pins")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "action",
        choices=(
            "gate",
            "dependencies",
            "pins",
            "tool-version",
            "installed",
            "performance-reference",
            "performance-scope",
        ),
    )
    parser.add_argument("--base")
    parser.add_argument("--tool")
    parser.add_argument("--tools", nargs="+")
    parser.add_argument("--base-checkout", type=Path)
    args = parser.parse_args()
    if args.action == "gate":
        require_success(json.loads(os.environ["CI_NEEDS"]))
    elif args.action == "pins":
        check_tool_pins()
    elif args.action == "tool-version":
        if not args.tool:
            parser.error("tool-version requires --tool")
        print(tool_version(args.tool))
    elif args.action == "installed":
        check_installed_tools(args.tools)
    elif args.action == "performance-reference":
        from .compare_performance import select_reference

        reference = select_reference(
            os.environ.get("PR_BASE_SHA", ""),
            os.environ.get("REQUESTED_BASE_REF", ""),
        )
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
            stream.write(f"reference={reference}\n")
    elif args.action == "performance-scope":
        select_performance_changes(args.base_checkout)
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
