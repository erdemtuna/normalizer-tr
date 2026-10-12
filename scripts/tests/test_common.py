"""Shared fixture loading and report IO."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.common import (
    check_source_revisions,
    policy_contract_fixture,
    source_revisions,
    write,
)


class CommonTests(unittest.TestCase):
    def test_policy_fixture_groups_preserve_order_and_fail_on_invalid_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixtures = root / "tests" / "fixtures"
            groups = fixtures / "policy-contract"
            groups.mkdir(parents=True)
            catalog = fixtures / "policy-contract.json"
            catalog.write_text(
                json.dumps(
                    ["policy-contract/second.json", "policy-contract/first.json"]
                ),
                encoding="utf-8",
            )
            first = groups / "first.json"
            second = groups / "second.json"
            first.write_text(json.dumps([{"id": "first"}]), encoding="utf-8")
            second.write_text(json.dumps([{"id": "second"}]), encoding="utf-8")
            cases, files = policy_contract_fixture(root)
            self.assertEqual(cases, [{"id": "second"}, {"id": "first"}])
            self.assertEqual(files, [catalog, second, first])
            first.write_text(json.dumps([{"id": "second"}]), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "unique nonempty"):
                policy_contract_fixture(root)
            first.unlink()
            with self.assertRaises(FileNotFoundError):
                policy_contract_fixture(root)

    def test_reports_refuse_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            write(path, {"status": "failed"})
            with self.assertRaises(FileExistsError):
                write(path, {"status": "passed"})
            self.assertEqual(json.loads(path.read_text()), {"status": "failed"})


class SourceRevisionTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.git("init", "--quiet", "--initial-branch=fixture")
        self.source = self.root / "source.rs"
        self.source.write_text("committed source\n", encoding="utf-8")
        (self.root / ".gitignore").write_text("/target/\n", encoding="utf-8")
        self.commit()

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-C", str(self.root), *args],
            text=True,
            stderr=subprocess.PIPE,
        ).strip()

    def commit(self):
        self.git("add", "source.rs", ".gitignore")
        self.git(
            "-c",
            "user.name=Source fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "Source fixture",
        )

    def test_clean_sources_allow_unrelated_untracked_and_ignored_outputs(self):
        (self.root / "notes.txt").write_text("unrelated", encoding="utf-8")
        (self.root / "target").mkdir()
        (self.root / "target" / "binary").write_bytes(b"generated")
        revisions = source_revisions(self.root, self.root)
        self.assertEqual(revisions, {self.root: self.git("rev-parse", "HEAD")})
        check_source_revisions(revisions)

    def test_dirty_tracked_sources_fail_before_capture(self):
        self.source.write_text("uncommitted source changes\n", encoding="utf-8")
        for staged in (False, True):
            with self.subTest(staged=staged):
                if staged:
                    self.git("add", "source.rs")
                with self.assertRaisesRegex(RuntimeError, "clean tracked sources"):
                    source_revisions(self.root)

    def test_tracked_edits_during_comparison_fail(self):
        revisions = source_revisions(self.root)
        self.source.write_text("edited while running\n", encoding="utf-8")
        with self.assertRaisesRegex(RuntimeError, "clean tracked sources"):
            check_source_revisions(revisions)

    def test_clean_revision_drift_during_comparison_fails(self):
        revisions = source_revisions(self.root)
        self.source.write_text("new committed source\n", encoding="utf-8")
        self.commit()
        with self.assertRaisesRegex(RuntimeError, "revision changed during"):
            check_source_revisions(revisions)

    def test_revision_drift_while_capturing_fails(self):
        with patch(
            "scripts.common.subprocess.check_output",
            side_effect=["old\n", "", "new\n"],
        ):
            with self.assertRaisesRegex(RuntimeError, "while capturing provenance"):
                source_revisions(self.root)
