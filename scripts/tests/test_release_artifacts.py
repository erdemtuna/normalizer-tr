"""Release gate regressions; no registry, native execution or credentials."""

import unittest
import hashlib
import io
import json
import tempfile
import subprocess
import urllib.error
from pathlib import Path
from unittest.mock import patch

from scripts.release_artifacts import (
    check_ci_run,
    check_members,
    check_metadata,
    check_registry_files,
    check_versions,
    download,
    fetch_json,
    preflight,
    verify_registry,
    wheel_platform,
)
from scripts.common import version

VERSION = version()

METADATA = (
    "Name: normalizer-tr\n"
    f"Version: {VERSION}\n"
    "Requires-Python: <3.15, >=3.11\n"
    "License-Expression: Apache-2.0\n"
)


class TimedOutBody(io.BytesIO):
    def read(self, size=-1):
        raise TimeoutError("response body timed out")


class TimedOutWriter(io.BytesIO):
    def write(self, value):
        raise TimeoutError("disk write timed out")


class ReleaseTests(unittest.TestCase):
    def test_versions_are_consistent(self):
        check_versions()

    def test_exact_main_ci_revision_is_required(self):
        run = {
            "head_sha": "abc",
            "head_branch": "main",
            "event": "push",
            "status": "completed",
            "conclusion": "success",
        }
        check_ci_run(run, "abc")
        for change in (
            {"head_sha": "other"},
            {"head_branch": "feature"},
            {"event": "pull_request"},
            {"status": "in_progress"},
            {"conclusion": "failure"},
            {"conclusion": "cancelled"},
        ):
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                check_ci_run({**run, **change}, "abc")

    def test_preflight_rejects_wrong_tag_and_unreviewed_history(self):
        with (
            patch(
                "scripts.release_artifacts.subprocess.check_output",
                return_value="abc\n",
            ),
            patch.dict(
                "scripts.release_artifacts.os.environ",
                {"GITHUB_REF": "refs/tags/v" + VERSION},
            ),
        ):
            with self.assertRaisesRegex(RuntimeError, "version tag"):
                preflight("v99.0.0", "owner/repo")
            with patch(
                "scripts.release_artifacts.subprocess.run",
                side_effect=[
                    None,
                    subprocess.CalledProcessError(1, ["git", "merge-base"]),
                ],
            ):
                with self.assertRaises(subprocess.CalledProcessError):
                    preflight("v" + VERSION, "owner/repo")

    def test_preflight_checks_tag_source_and_latest_ci(self):
        head = "abc"
        success = {
            "head_sha": head,
            "head_branch": "main",
            "event": "push",
            "status": "completed",
            "conclusion": "success",
            "html_url": "https://ci/run",
        }
        with patch.dict(
            "scripts.release_artifacts.os.environ",
            {"GITHUB_REF": "refs/tags/v" + VERSION},
        ):
            for runs, error in (
                ([], "no main CI"),
                ([{**success, "conclusion": "failure"}], "not successful"),
            ):
                with (
                    patch(
                        "scripts.release_artifacts.subprocess.check_output",
                        side_effect=[head, head, json.dumps({"workflow_runs": runs})],
                    ),
                    patch("scripts.release_artifacts.subprocess.run"),
                    self.assertRaisesRegex(RuntimeError, error),
                ):
                    preflight("v" + VERSION, "owner/repo")
            with (
                patch(
                    "scripts.release_artifacts.subprocess.check_output",
                    side_effect=[head, "different"],
                ),
                self.assertRaisesRegex(RuntimeError, "checked-out source"),
            ):
                preflight("v" + VERSION, "owner/repo")
            with (
                patch(
                    "scripts.release_artifacts.subprocess.check_output",
                    side_effect=[head, head, json.dumps({"workflow_runs": [success]})],
                ),
                patch("scripts.release_artifacts.subprocess.run"),
            ):
                preflight("v" + VERSION, "owner/repo")

    def test_registry_requires_exact_names_and_checksums(self):
        expected = [{"name": "wheel.whl", "sha256": "abc"}]
        check_registry_files(expected, expected)
        for actual in (
            [],
            expected * 2,
            [{"name": "wheel.whl", "sha256": "different"}],
        ):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                check_registry_files(expected, actual)

    def test_download_checks_actual_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            data = b"reviewed bytes"
            with patch(
                "scripts.release_artifacts.urllib.request.urlopen",
                return_value=io.BytesIO(data),
            ):
                download(
                    "https://registry/file",
                    Path(directory) / "good",
                    hashlib.sha256(data).hexdigest(),
                )
            with patch(
                "scripts.release_artifacts.urllib.request.urlopen",
                return_value=io.BytesIO(data),
            ):
                with self.assertRaisesRegex(RuntimeError, "bytes differ"):
                    download("https://registry/file", Path(directory) / "bad", "wrong")

    def test_registry_status_is_not_a_success_shaped_missing_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "release-manifest.json").write_text(
                json.dumps(
                    {
                        "version": VERSION,
                        "python_artifacts": [{"name": "wheel.whl", "sha256": "abc"}],
                    }
                )
            )
            error = urllib.error.HTTPError(
                "https://registry", 404, "missing", None, None
            )
            with patch("scripts.release_artifacts.fetch_json", side_effect=error):
                with self.assertRaisesRegex(RuntimeError, "not all verified"):
                    verify_registry(root, ["pypi"], root / "missing")
            result = json.loads(
                (root / "missing/registry-verification.json").read_text()
            )
            self.assertEqual(result["registries"]["pypi"]["status"], "not-published")
            metadata = {
                "urls": [
                    {
                        "filename": "wheel.whl",
                        "digests": {"sha256": "abc"},
                        "url": "https://registry",
                    }
                ]
            }
            with (
                patch("scripts.release_artifacts.fetch_json", return_value=metadata),
                patch("scripts.release_artifacts.download", side_effect=error),
            ):
                with self.assertRaises(RuntimeError):
                    verify_registry(root, ["pypi"], root / "bad-file")
            result = json.loads(
                (root / "bad-file/registry-verification.json").read_text()
            )
            self.assertEqual(result["registries"]["pypi"]["status"], "failed")

    def test_response_read_timeouts_are_explicit_network_failures(self):
        with patch(
            "scripts.release_artifacts.urllib.request.urlopen",
            return_value=TimedOutBody(),
        ):
            with self.assertRaisesRegex(
                RuntimeError, "registry response read timed out"
            ):
                fetch_json("https://registry/metadata")
        with tempfile.TemporaryDirectory() as directory:
            with patch(
                "scripts.release_artifacts.urllib.request.urlopen",
                return_value=TimedOutBody(),
            ):
                with self.assertRaisesRegex(
                    RuntimeError, "registry response read timed out"
                ):
                    download(
                        "https://registry/file", Path(directory) / "partial", "abc"
                    )

    def test_metadata_and_artifact_timeouts_preserve_both_registry_results(self):
        for timed_out in ("metadata", "artifact"):
            with (
                self.subTest(timed_out=timed_out),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                wheel, core = b"reviewed wheel", b"reviewed core"
                wheel_sha = hashlib.sha256(wheel).hexdigest()
                core_sha = hashlib.sha256(core).hexdigest()
                record = {
                    "version": VERSION,
                    "python_artifacts": [{"name": "wheel.whl", "sha256": wheel_sha}],
                    "core": {"name": "normalizer-tr.crate", "sha256": core_sha},
                }
                (root / "release-manifest.json").write_text(
                    json.dumps(record), encoding="utf-8"
                )
                requested = []

                def response(url, timeout):
                    requested.append(url)
                    if "pypi.org/pypi/" in url:
                        if timed_out == "metadata":
                            return TimedOutBody()
                        return io.BytesIO(
                            json.dumps(
                                {
                                    "urls": [
                                        {
                                            "filename": "wheel.whl",
                                            "digests": {"sha256": wheel_sha},
                                            "url": "https://registry/wheel",
                                        }
                                    ]
                                }
                            ).encode("utf-8")
                        )
                    if url == "https://registry/wheel":
                        return TimedOutBody()
                    if "crates.io/api/" in url:
                        return io.BytesIO(
                            json.dumps({"version": {"checksum": core_sha}}).encode(
                                "utf-8"
                            )
                        )
                    if "static.crates.io/" in url:
                        return io.BytesIO(core)
                    raise AssertionError(f"unexpected URL: {url}")

                output = root / "results"
                with (
                    patch(
                        "scripts.release_artifacts.urllib.request.urlopen",
                        side_effect=response,
                    ),
                    patch("scripts.release_artifacts.registry_consumer") as consume,
                    self.assertRaisesRegex(RuntimeError, "not all verified"),
                ):
                    verify_registry(root, ["pypi", "crates"], output)
                results = json.loads(
                    (output / "registry-verification.json").read_text()
                )["registries"]
                self.assertEqual(results["pypi"]["status"], "failed")
                self.assertIn("response read timed out", results["pypi"]["error"])
                self.assertEqual(results["crates"], {"status": "verified", "files": 1})
                self.assertTrue(any("pypi.org/pypi/" in url for url in requested))
                self.assertTrue(any("crates.io/api/" in url for url in requested))
                consume.assert_called_once_with("crates", output / "crates", record)

    def test_disk_write_timeout_is_not_disguised_as_network_failure(self):
        with (
            patch(
                "scripts.release_artifacts.urllib.request.urlopen",
                return_value=io.BytesIO(b"bytes"),
            ),
            patch.object(Path, "open", return_value=TimedOutWriter()),
            self.assertRaisesRegex(TimeoutError, "disk write timed out"),
        ):
            download("https://registry/file", Path("destination"), "abc")

    def test_consumer_failure_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "release-manifest.json").write_text(
                json.dumps(
                    {
                        "version": VERSION,
                        "python_artifacts": [{"name": "wheel.whl", "sha256": "abc"}],
                    }
                )
            )
            metadata = {
                "urls": [
                    {
                        "filename": "wheel.whl",
                        "digests": {"sha256": "abc"},
                        "url": "https://registry",
                    }
                ]
            }
            with (
                patch("scripts.release_artifacts.fetch_json", return_value=metadata),
                patch("scripts.release_artifacts.download"),
                patch(
                    "scripts.release_artifacts.registry_consumer",
                    side_effect=RuntimeError("bad consumer"),
                ),
            ):
                with self.assertRaises(RuntimeError):
                    verify_registry(root, ["pypi"], root / "failed")
            result = json.loads(
                (root / "failed/registry-verification.json").read_text()
            )
            self.assertEqual(result["registries"]["pypi"]["status"], "failed")

    def test_metadata_accepts_normalized_requirement_order_and_spacing(self):
        check_metadata(METADATA.encode())

    def test_metadata_cannot_weaken_the_matrix_or_add_runtime_dependencies(self):
        for text in (
            METADATA.replace(">=3.11", ">=3.10"),
            METADATA.replace("normalizer-tr", "different-package"),
            METADATA.replace(f"Version: {VERSION}", "Version: 99.0.0"),
            METADATA + "Requires-Dist: torch\n",
        ):
            with self.subTest(text=text), self.assertRaises(RuntimeError):
                check_metadata(text.encode())

    def test_archive_excludes_secrets_models_and_cross_platform_traversal(self):
        check_members(["normalizer_tr/__init__.py", "licenses/COPYRIGHT-library.html"])
        for name in (
            "../outside",
            r"..\outside",
            r"C:\outside",
            "/outside",
            "normalizer_tr/.env.production",
            "credentials.toml",
            "integrations/example.py",
            "decoder.pt",
            ".git/config",
        ):
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                check_members([name])

    def test_platform_matrix_does_not_claim_other_architectures(self):
        for tag, expected in (
            ("win_amd64", "windows"),
            ("manylinux_2_28_x86_64", "linux"),
            ("macosx_12_0_x86_64", "macos-x64"),
            ("macosx_12_0_arm64", "macos-arm64"),
        ):
            self.assertEqual(wheel_platform(tag), expected)
        for tag in ("win_arm64", "musllinux_1_2_x86_64", "manylinux_2_28_aarch64"):
            with self.subTest(tag=tag), self.assertRaises(RuntimeError):
                wheel_platform(tag)


if __name__ == "__main__":
    unittest.main()
