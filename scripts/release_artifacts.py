"""Inspect the exact release set; registry authentication is not handled here."""

import argparse
import email.parser
import itertools
import json
import os
import platform
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tomllib
import urllib.error
import urllib.request
import zipfile

from .common import ROOT, sha, version, write
from .verification import inspect_wheel

PYTHONS = ("3.11", "3.12", "3.13", "3.14")
PLATFORMS = ("windows", "linux", "macos-x64", "macos-arm64")
NETWORK_TIMEOUT = 30
DOWNLOAD_CHUNK_BYTES = 1024 * 1024


def check_versions():
    release = version()
    binding = tomllib.loads(
        (ROOT / "bindings/python/Cargo.toml").read_text(encoding="utf-8")
    )
    python = tomllib.loads(
        (ROOT / "bindings/python/pyproject.toml").read_text(encoding="utf-8")
    )
    if (
        binding["package"]["version"] != release
        or binding["dependencies"]["normalizer-tr"]["version"] != "=" + release
        or python["project"]["version"] != release
    ):
        raise RuntimeError("core, companion, and Python versions must agree")


def check_ci_run(run, head):
    if (
        run.get("head_sha") != head
        or run.get("head_branch") != "main"
        or run.get("event") != "push"
        or run.get("status") != "completed"
        or run.get("conclusion") != "success"
    ):
        raise RuntimeError("latest main CI run for the tagged SHA is not successful")


def preflight(tag, repository):
    check_versions()
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if tag != "v" + version():
        raise RuntimeError("publishing requires the matching version tag")
    if os.environ.get("GITHUB_REF", f"refs/tags/{tag}") != f"refs/tags/{tag}":
        raise RuntimeError("publish dispatch must select a tag, not a branch")
    tagged = subprocess.check_output(
        ["git", "rev-parse", f"refs/tags/{tag}^{{commit}}"], text=True
    ).strip()
    if tagged != head:
        raise RuntimeError("tag does not identify the checked-out source")
    subprocess.run(["git", "fetch", "--no-tags", "origin", "main"], check=True)
    subprocess.run(
        ["git", "merge-base", "--is-ancestor", head, "origin/main"], check=True
    )
    payload = json.loads(
        subprocess.check_output(
            [
                "gh",
                "api",
                f"repos/{repository}/actions/workflows/ci.yml/runs?head_sha={head}&event=push&per_page=1",
            ],
            text=True,
        )
    )
    if not payload["workflow_runs"]:
        raise RuntimeError("no main CI run exists for the tagged SHA")
    run = payload["workflow_runs"][0]
    check_ci_run(run, head)
    print(f"Publication preflight passed: {head}, CI {run['html_url']}")


def check_registry_files(expected, published):
    actual = {row["name"]: row for row in published}
    if len(actual) != len(published) or actual.keys() != {
        row["name"] for row in expected
    }:
        raise RuntimeError("registry file set differs from the release manifest")
    for row in expected:
        if actual[row["name"]]["sha256"] != row["sha256"]:
            raise RuntimeError(f"registry checksum differs: {row['name']}")
    return actual


def read_response(response, size=-1):
    try:
        return response.read(size)
    except TimeoutError as error:
        raise RuntimeError(f"registry response read timed out: {error}") from error


def fetch_json(url):
    with urllib.request.urlopen(url, timeout=NETWORK_TIMEOUT) as response:
        return json.loads(read_response(response))


def download(url, path, expected_sha):
    with (
        urllib.request.urlopen(url, timeout=NETWORK_TIMEOUT) as response,
        path.open("xb") as stream,
    ):
        while chunk := read_response(response, DOWNLOAD_CHUNK_BYTES):
            stream.write(chunk)
    if sha(path) != expected_sha:
        raise RuntimeError(f"downloaded bytes differ from manifest: {path.name}")


def registry_consumer(registry, directory, record):
    if registry == "pypi":
        system = {"Windows": "windows", "Linux": "linux", "Darwin": "macos"}
        host = system[platform.system()]
        if host == "macos":
            host += (
                "-arm64"
                if platform.machine().lower() in {"arm64", "aarch64"}
                else "-x64"
            )
        interpreter = f"{sys.version_info.major}.{sys.version_info.minor}"
        wheels = [
            row
            for row in record["python_artifacts"]
            if row["kind"] == "wheel"
            and row["platform"] == host
            and row["python"] == interpreter
        ]
        if len(wheels) != 1:
            raise RuntimeError("no unique tested wheel matches the consumer host")
        env = directory / "consumer"
        subprocess.run([sys.executable, "-m", "venv", str(env)], check=True)
        python = env / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
        subprocess.run(
            [
                str(python),
                "-m",
                "pip",
                "install",
                "--no-index",
                "--no-deps",
                str(directory / wheels[0]["name"]),
            ],
            check=True,
        )
        (directory / "reports").mkdir()
        subprocess.run(
            [
                str(python),
                "-m",
                "scripts.verification",
                "consume",
                str(directory),
            ],
            cwd=ROOT,
            check=True,
        )
        consumer = json.loads(
            (directory / "reports/python-consumer.json").read_text(encoding="utf-8")
        )
        if consumer["normalizer_id"] != "normalizer-tr/" + record["version"]:
            raise RuntimeError(
                "installed registry consumer identity differs from release"
            )
    else:
        manifest = directory / "Cargo.toml"
        manifest.write_text(
            "[workspace]\n[package]\nname='registry-consumer'\nversion='0.0.0'\nedition='2024'\npublish=false\n"
            "[lib]\n"
            f"path={json.dumps((ROOT / 'scripts/rust/core_consumer.rs').as_posix())}\n"
            "[features]\ndefault=[]\nserde=['normalizer-tr/serde']\n"
            f"[dependencies]\nnormalizer-tr='={record['version']}'\n",
            encoding="utf-8",
        )
        env = {**os.environ, "CARGO_TARGET_DIR": str(directory / "target")}
        subprocess.run(
            ["cargo", "generate-lockfile", "--manifest-path", str(manifest)],
            env=env,
            check=True,
        )
        lock = tomllib.loads((directory / "Cargo.lock").read_text(encoding="utf-8"))
        packages = [row for row in lock["package"] if row["name"] == "normalizer-tr"]
        if len(packages) != 1 or packages[0]["checksum"] != record["core"]["sha256"]:
            raise RuntimeError(
                "registry consumer did not resolve the reviewed core archive"
            )
        for features in ([], ["--all-features"]):
            subprocess.run(
                [
                    "cargo",
                    "test",
                    "--locked",
                    "--manifest-path",
                    str(manifest),
                    *features,
                ],
                env=env,
                check=True,
            )


def verify_registry(directory, registries, output):
    record = json.loads(
        (directory / "release-manifest.json").read_text(encoding="utf-8")
    )
    if record["version"] != version():
        raise RuntimeError(
            "check out the release version before verifying its consumers"
        )
    output.mkdir()
    results = {}
    for registry in registries:
        destination = output / registry
        destination.mkdir()
        found = False
        try:
            if registry == "pypi":
                metadata = fetch_json(
                    f"https://pypi.org/pypi/normalizer-tr/{record['version']}/json"
                )
                expected = record["python_artifacts"]
                published = [
                    {
                        "name": row["filename"],
                        "sha256": row["digests"]["sha256"],
                        "url": row["url"],
                    }
                    for row in metadata["urls"]
                ]
            else:
                metadata = fetch_json(
                    f"https://crates.io/api/v1/crates/normalizer-tr/{record['version']}"
                )
                expected = [record["core"]]
                published = [
                    {
                        "name": record["core"]["name"],
                        "sha256": metadata["version"]["checksum"],
                        "url": f"https://static.crates.io/crates/normalizer-tr/{record['core']['name']}",
                    }
                ]
            found = True
            actual = check_registry_files(expected, published)
            for row in expected:
                download(
                    actual[row["name"]]["url"], destination / row["name"], row["sha256"]
                )
            registry_consumer(registry, destination, record)
            results[registry] = {"status": "verified", "files": len(expected)}
        except urllib.error.HTTPError as error:
            results[registry] = {
                "status": "not-published"
                if error.code == 404 and not found
                else "failed",
                "error": str(error),
            }
        except (
            urllib.error.URLError,
            RuntimeError,
            subprocess.CalledProcessError,
            json.JSONDecodeError,
        ) as error:
            results[registry] = {"status": "failed", "error": str(error)}
    write(
        output / "registry-verification.json",
        {"version": record["version"], "registries": results},
    )
    print(json.dumps(results, indent=2))
    if any(row["status"] != "verified" for row in results.values()):
        raise RuntimeError(
            "requested registries are not all verified; see registry-verification.json"
        )


def check_members(names):
    for name in names:
        path = PurePosixPath(name)
        if (
            path.is_absolute()
            or "\\" in name
            or re.match(r"^[A-Za-z]:", name)
            or ".." in path.parts
            or set(path.parts)
            & {
                ".git",
                "target",
                "__pycache__",
                "integrations",
                ".venv",
                ".pytest_cache",
            }
            or path.suffix.lower() in {".pt", ".wav", ".exe", ".pem", ".key"}
            or path.name.startswith(".env")
            or path.name in {".pypirc", "credentials.toml"}
        ):
            raise RuntimeError(f"excluded or unsafe release entry: {name}")


def check_metadata(data):
    metadata = email.parser.Parser().parsestr(data.decode("utf-8"))
    if metadata["Name"] != "normalizer-tr" or metadata["Version"] != version():
        raise RuntimeError("distribution name/version differs from the core")
    requirement = metadata["Requires-Python"]
    if requirement is None or {part.strip() for part in requirement.split(",")} != {
        ">=3.11",
        "<3.15",
    }:
        raise RuntimeError("Python requirement differs from the tested matrix")
    if metadata.get_all("Requires-Dist"):
        raise RuntimeError("standalone binding acquired a runtime dependency")
    if metadata["License-Expression"] != "Apache-2.0":
        raise RuntimeError("owned license metadata differs")


def wheel_platform(tag):
    if tag == "win_amd64":
        return "windows"
    if "manylinux_2_28_x86_64" in tag.split("."):
        return "linux"
    if re.fullmatch(r"macosx_\d+_\d+_x86_64", tag):
        return "macos-x64"
    if re.fullmatch(r"macosx_\d+_\d+_arm64", tag):
        return "macos-arm64"
    raise RuntimeError(f"unapproved wheel platform tag: {tag}")


def inspect_python(path):
    if path.suffix == ".whl":
        inspect_wheel(path)
        match = re.fullmatch(
            r"normalizer_tr-(\d+\.\d+\.\d+)-(cp3\d{2})-\2-(.+)\.whl", path.name
        )
        if match is None or match[1] != version():
            raise RuntimeError(f"unexpected wheel filename: {path.name}")
        python = "3." + match[2][3:]
        if python not in PYTHONS:
            raise RuntimeError("wheel targets an unapproved interpreter")
        with zipfile.ZipFile(path) as archive:
            names = archive.namelist()
            check_members(names)
            check_metadata(
                archive.read(next(n for n in names if n.endswith("/METADATA")))
            )
            native = [n for n in names if n.endswith((".pyd", ".so"))]
            if len(native) != 1 or not native[0].startswith("normalizer_tr/_native."):
                raise RuntimeError("expected exactly one native normalizer extension")
        return {"kind": "wheel", "python": python, "platform": wheel_platform(match[3])}
    if path.name == f"normalizer_tr-{version()}.tar.gz":
        with tarfile.open(path) as archive:
            names = archive.getnames()
            check_members(names)
            infos = [n for n in names if n.endswith("/PKG-INFO")]
            if len(infos) != 1:
                raise RuntimeError("expected one source-distribution metadata file")
            check_metadata(archive.extractfile(infos[0]).read())
            if (
                sum(n.endswith("/Cargo.toml") for n in names) < 2
                or not any(n.endswith("/normalizer_tr/__init__.py") for n in names)
                or not any("LICENSE-UNICODE" in n for n in names)
            ):
                raise RuntimeError("sdist lacks the core, binding or required rights")
        return {"kind": "sdist"}
    raise RuntimeError(f"unexpected Python release artifact: {path.name}")


def artifact_record(path):
    return {
        "name": path.name,
        "sha256": sha(path),
        "bytes": path.stat().st_size,
        **inspect_python(path),
    }


def inspect(directory, python, platform, sdist):
    paths = sorted(directory.iterdir())
    if len(paths) != 1:
        raise RuntimeError("each build job must produce exactly one distribution")
    record = artifact_record(paths[0])
    if sdist:
        if record["kind"] != "sdist":
            raise RuntimeError("expected an sdist")
    elif (
        record["kind"] != "wheel"
        or record["python"] != python
        or record["platform"] != platform
    ):
        raise RuntimeError("built wheel differs from its test job")
    print(json.dumps(record, indent=2))


def manifest(directory, core_directory, output):
    artifacts = [artifact_record(path) for path in sorted(directory.iterdir())]
    actual = [
        (record["python"], record["platform"])
        for record in artifacts
        if record["kind"] == "wheel"
    ]
    expected = list(itertools.product(PYTHONS, PLATFORMS))
    if sorted(actual) != sorted(expected):
        raise RuntimeError("release wheel matrix is missing, duplicated or unexpected")
    if sum(record["kind"] == "sdist" for record in artifacts) != 1:
        raise RuntimeError("release requires exactly one tested sdist")
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    core = list(core_directory.glob(f"normalizer-tr-{version()}.crate"))
    if len(core) != 1:
        raise RuntimeError("expected one core archive")
    with tarfile.open(core[0]) as archive:
        names = archive.getnames()
        check_members(names)
        vcs = json.load(
            archive.extractfile(
                next(n for n in names if n.endswith(".cargo_vcs_info.json"))
            )
        )
        metadata = tomllib.loads(
            archive.extractfile(f"normalizer-tr-{version()}/Cargo.toml")
            .read()
            .decode("utf-8")
        )
        if vcs["git"]["sha1"] != head or metadata["package"]["publish"] != [
            "crates-io"
        ]:
            raise RuntimeError("core archive provenance/publication target differs")
    write(
        output,
        {
            "schema_version": 1,
            "version": version(),
            "source_head": head,
            "workflow_run_id": os.environ.get("GITHUB_RUN_ID"),
            "python_artifacts": artifacts,
            "core": {
                "name": core[0].name,
                "sha256": sha(core[0]),
                "bytes": core[0].stat().st_size,
                "vcs": vcs,
            },
            "matrix": {"python": PYTHONS, "platforms": PLATFORMS},
            "rust_toolchain": "1.99.0",
            "linux_baseline": "glibc 2.28 / manylinux_2_28",
        },
    )


def verify(directory, tag):
    record = json.loads(
        (directory / "release-manifest.json").read_text(encoding="utf-8")
    )
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if (
        tag != "v" + version()
        or record["version"] != version()
        or record["source_head"] != head
    ):
        raise RuntimeError("release tag, version or source revision differs")
    paths = sorted((directory / "dist").iterdir())
    actual = [artifact_record(path) for path in paths]
    if actual != record["python_artifacts"]:
        raise RuntimeError("publish artifacts differ from the verified build set")
    core = directory / "target" / "release-core" / record["core"]["name"]
    if sha(core) != record["core"]["sha256"]:
        raise RuntimeError("core archive differs from the verified build")
    print(f"Verified {len(actual)} Python artifacts and core at {head}")


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="action", required=True)
    commands.add_parser("versions")
    check = commands.add_parser("inspect")
    check.add_argument("directory", type=Path)
    check.add_argument("--python", choices=PYTHONS)
    check.add_argument("--platform", choices=PLATFORMS)
    check.add_argument("--sdist", action="store_true")
    collect = commands.add_parser("manifest")
    collect.add_argument("directory", type=Path)
    collect.add_argument("--core", type=Path, required=True)
    collect.add_argument("--output", type=Path, required=True)
    publish = commands.add_parser("verify")
    publish.add_argument("directory", type=Path)
    publish.add_argument("--tag", required=True)
    before = commands.add_parser("preflight")
    before.add_argument("--tag", required=True)
    before.add_argument("--repository", required=True)
    registry = commands.add_parser("registry")
    registry.add_argument("directory", type=Path)
    registry.add_argument(
        "--registry", action="append", choices=("pypi", "crates"), required=True
    )
    registry.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.action == "inspect":
        inspect(args.directory, args.python, args.platform, args.sdist)
    elif args.action == "manifest":
        manifest(args.directory, args.core, args.output)
    elif args.action == "verify":
        verify(args.directory, args.tag)
    elif args.action == "preflight":
        preflight(args.tag, args.repository)
    elif args.action == "versions":
        check_versions()
    else:
        verify_registry(args.directory.resolve(), args.registry, args.output.resolve())


if __name__ == "__main__":
    main()
