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
    main,
    PUBLICATION_MAX_ATTEMPTS,
    PUBLICATION_RETRY_DELAY_SECONDS,
    verify_registry,
    wheel_platform,
)
from scripts.common import ROOT, version

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


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.expected = [
            {"name": "first.whl", "sha256": "first"},
            {"name": "second.whl", "sha256": "second"},
        ]
        (self.root / "release-manifest.json").write_text(
            json.dumps(
                {
                    "version": VERSION,
                    "python_artifacts": self.expected,
                    "core": {"name": "core.crate", "sha256": "core"},
                }
            ),
            encoding="utf-8",
        )
        self.fetch = self.enterContext(patch("scripts.release_artifacts.fetch_json"))
        self.download = self.enterContext(patch("scripts.release_artifacts.download"))
        self.consume = self.enterContext(
            patch("scripts.release_artifacts.registry_consumer")
        )
        self.sleep = self.enterContext(patch("scripts.release_artifacts.time.sleep"))
        self.log = self.enterContext(patch("sys.stderr", new_callable=io.StringIO))
        self.output_number = 0

    def metadata(self, rows=None):
        return {
            "urls": [
                {
                    "filename": row["name"],
                    "digests": {"sha256": row["sha256"]},
                    "url": "https://registry/" + row["name"],
                }
                for row in (self.expected if rows is None else rows)
            ]
        }

    def verify(self, *, wait=True, registries=None):
        self.output_number += 1
        self.output = self.root / str(self.output_number)
        verify_registry(
            self.root,
            registries or ["pypi"],
            self.output,
            wait_for_publication=wait,
        )

    def results(self):
        return json.loads(
            (self.output / "registry-verification.json").read_text(encoding="utf-8")
        )["registries"]

    def test_complete_metadata_does_not_delay_or_repeat_work(self):
        self.fetch.return_value = self.metadata()
        self.verify()
        self.fetch.assert_called_once()
        self.sleep.assert_not_called()
        self.assertEqual(self.download.call_count, len(self.expected))
        self.consume.assert_called_once()
        self.assertEqual(self.results()["pypi"], {"status": "verified", "files": 2})

    def test_missing_release_and_partial_metadata_wait_before_work(self):
        missing = urllib.error.HTTPError("https://registry", 404, "missing", None, None)
        responses = iter([missing, self.metadata(self.expected[:1]), self.metadata()])

        def metadata_response(url):
            self.download.assert_not_called()
            self.consume.assert_not_called()
            response = next(responses)
            if isinstance(response, Exception):
                raise response
            return response

        self.fetch.side_effect = metadata_response
        self.verify()
        self.assertEqual(self.fetch.call_count, 3)
        self.assertEqual(
            self.sleep.call_args_list,
            [unittest.mock.call(PUBLICATION_RETRY_DELAY_SECONDS)] * 2,
        )
        self.assertIn("second.whl", self.log.getvalue())
        self.assertEqual(self.download.call_count, len(self.expected))
        self.consume.assert_called_once()

    def test_integrity_errors_never_wait_even_with_missing_files(self):
        for rows, message in (
            (self.expected[:1] * 2, "duplicate"),
            ([{"name": "unexpected.whl", "sha256": "first"}], "unexpected"),
            ([{"name": "first.whl", "sha256": "corrupt"}], "checksum"),
        ):
            with self.subTest(message=message):
                self.fetch.reset_mock()
                self.fetch.return_value = self.metadata(rows)
                with self.assertRaisesRegex(RuntimeError, "not all verified"):
                    self.verify()
                self.fetch.assert_called_once()
                self.assertIn(message, self.results()["pypi"]["error"])
                self.sleep.assert_not_called()
                self.download.assert_not_called()
                self.consume.assert_not_called()

    def test_default_verification_remains_fail_fast(self):
        for response, status in (
            (self.metadata([]), "failed"),
            (
                urllib.error.HTTPError("https://registry", 404, "missing", None, None),
                "not-published",
            ),
        ):
            with self.subTest(status=status):
                self.fetch.reset_mock()
                self.fetch.side_effect = (
                    response if isinstance(response, Exception) else None
                )
                self.fetch.return_value = response
                with self.assertRaisesRegex(RuntimeError, "not all verified"):
                    self.verify(wait=False)
                self.fetch.assert_called_once()
                self.sleep.assert_not_called()
                self.assertEqual(self.results()["pypi"]["status"], status)

    def test_exhausted_partial_metadata_preserves_other_registry_results(self):
        self.fetch.side_effect = [self.metadata(self.expected[:1])] * (
            PUBLICATION_MAX_ATTEMPTS
        ) + [{"version": {"checksum": "core"}}]
        with self.assertRaisesRegex(RuntimeError, "not all verified"):
            self.verify(registries=["pypi", "crates"])
        self.assertEqual(self.fetch.call_count, PUBLICATION_MAX_ATTEMPTS + 1)
        self.assertEqual(
            self.sleep.call_args_list,
            [unittest.mock.call(PUBLICATION_RETRY_DELAY_SECONDS)]
            * (PUBLICATION_MAX_ATTEMPTS - 1),
        )
        self.assertEqual(self.results()["pypi"]["status"], "failed")
        self.assertIn("second.whl", self.results()["pypi"]["error"])
        self.assertEqual(self.results()["crates"], {"status": "verified", "files": 1})
        self.download.assert_called_once()
        self.consume.assert_called_once()
        self.assertEqual(self.consume.call_args.args[0], "crates")

    def test_exhausted_missing_release_is_not_published(self):
        self.fetch.side_effect = urllib.error.HTTPError(
            "https://registry", 404, "missing", None, None
        )
        with self.assertRaisesRegex(RuntimeError, "not all verified"):
            self.verify()
        self.assertEqual(self.fetch.call_count, PUBLICATION_MAX_ATTEMPTS)
        self.assertEqual(self.sleep.call_count, PUBLICATION_MAX_ATTEMPTS - 1)
        self.assertEqual(self.results()["pypi"]["status"], "not-published")
        self.download.assert_not_called()
        self.consume.assert_not_called()

    def test_other_metadata_failures_are_not_retried(self):
        failures = [
            urllib.error.HTTPError("https://registry", code, "failure", None, None)
            for code in (403, 429, 500)
        ] + [
            urllib.error.URLError("TLS handshake failed"),
            json.JSONDecodeError("invalid metadata", "", 0),
            RuntimeError("registry response read timed out"),
        ]
        for error in failures:
            with self.subTest(error=error):
                self.fetch.reset_mock()
                self.fetch.side_effect = error
                with self.assertRaisesRegex(RuntimeError, "not all verified"):
                    self.verify()
                self.fetch.assert_called_once()
                self.sleep.assert_not_called()
                self.assertEqual(self.results()["pypi"]["status"], "failed")
                self.download.assert_not_called()
                self.consume.assert_not_called()

    def test_download_and_consumer_failures_do_not_restart_admission(self):
        self.fetch.return_value = self.metadata()
        for target in (self.download, self.consume):
            with self.subTest(target=target):
                self.fetch.reset_mock()
                self.download.reset_mock(side_effect=True)
                self.consume.reset_mock(side_effect=True)
                target.side_effect = urllib.error.HTTPError(
                    "https://registry", 404, "missing", None, None
                )
                with self.assertRaisesRegex(RuntimeError, "not all verified"):
                    self.verify()
                self.fetch.assert_called_once()
                self.sleep.assert_not_called()
                self.assertEqual(self.results()["pypi"]["status"], "failed")

    def test_malformed_metadata_is_reported_without_waiting(self):
        for metadata in (
            None,
            [],
            {},
            {"urls": None},
            {"urls": [None]},
            {"urls": [{"filename": "first.whl", "url": "https://registry"}]},
        ):
            with self.subTest(metadata=metadata):
                self.fetch.reset_mock()
                self.fetch.return_value = metadata
                with self.assertRaisesRegex(RuntimeError, "not all verified"):
                    self.verify()
                self.fetch.assert_called_once()
                self.sleep.assert_not_called()
                self.download.assert_not_called()
                self.consume.assert_not_called()
                self.assertIn("metadata", self.results()["pypi"]["error"])

    def test_waiting_requires_pypi(self):
        with self.assertRaisesRegex(RuntimeError, "requires the PyPI"):
            self.verify(registries=["crates"])
        self.fetch.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_cli_passes_wait_flag_explicitly(self):
        with (
            patch(
                "sys.argv",
                [
                    "release-artifacts",
                    "registry",
                    str(self.root),
                    "--registry",
                    "pypi",
                    "--output",
                    str(self.root / "out"),
                    "--wait-for-publication",
                ],
            ),
            patch("scripts.release_artifacts.verify_registry") as verify,
        ):
            main()
        verify.assert_called_once_with(
            self.root.resolve(),
            ["pypi"],
            (self.root / "out").resolve(),
            wait_for_publication=True,
        )

    def test_workflow_waits_only_after_upload_and_isolates_recovery(self):
        workflow = (ROOT / ".github/workflows/release.yml").read_text()
        publish = workflow.split("\n  publish:", 1)[1].split(
            "\n  verify-published:", 1
        )[0]
        recovery = workflow.split("\n  verify-published:", 1)[1]
        self.assertIn("if: inputs.verify_run == ''", workflow)
        self.assertIn("if: inputs.verify_run == '' && inputs.publish", publish)
        self.assertIn("--registry pypi --wait-for-publication", publish)
        self.assertIn("if: inputs.verify_run != ''", recovery)
        self.assertIn('"$PUBLISH" != "false"', recovery)
        self.assertIn("actions: read", recovery)
        self.assertNotIn("id-token", recovery)
        self.assertNotIn("pypa/gh-action-pypi-publish", recovery)
        self.assertNotIn("--wait-for-publication", recovery)


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
