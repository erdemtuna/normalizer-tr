"""Shared fixture loading and report IO."""

import json
import tempfile
import unittest
from pathlib import Path

from scripts.common import policy_contract_fixture, write


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
