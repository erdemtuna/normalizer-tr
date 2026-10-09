"""Verification machinery checks; no models or external source access."""

import json
import tarfile
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch
from verification import (
    write,
    wheel,
    prepare,
    policy_contract_fixture,
    validate_policy_contract_measurement,
)


class VerificationTests(unittest.TestCase):
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
            with patch("verification.ROOT", root):
                cases, files = policy_contract_fixture()
                self.assertEqual(cases, [{"id": "second"}, {"id": "first"}])
                self.assertEqual(files, [catalog, second, first])
                first.write_text(json.dumps([{"id": "second"}]), encoding="utf-8")
                with self.assertRaisesRegex(RuntimeError, "unique nonempty"):
                    policy_contract_fixture()
                first.unlink()
                with self.assertRaises(FileNotFoundError):
                    policy_contract_fixture()

    def test_policy_contract_reports_require_every_policy_and_reviewed_input(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixtures = root / "tests" / "fixtures"
            fixtures.mkdir(parents=True)
            cases = [
                {
                    "id": "money",
                    "class": "money",
                    "text": "25TL",
                    "expected": "yirmi beş Türk lirası",
                }
            ]
            (fixtures / "policy-contract.json").write_text(
                json.dumps(["policy-contract/money.json"]), encoding="utf-8"
            )
            group = fixtures / "policy-contract"
            group.mkdir()
            (group / "money.json").write_text(json.dumps(cases), encoding="utf-8")
            keys = [
                f"{cohort}:{policy}"
                for cohort in ("short", "medium")
                for policy in ("preserve", "reject", "fallback")
            ]
            value = {
                "input_cases": cases,
                "cohorts": {key: {"count": 10000} for key in keys},
                "per_class_policy": {
                    key.replace(":", ":money:"): {"count": 10000} for key in keys
                },
            }
            with patch("verification.ROOT", root):
                validate_policy_contract_measurement(value)
                broken = {**value, "input_cases": []}
                with self.assertRaisesRegex(RuntimeError, "reviewed fixture"):
                    validate_policy_contract_measurement(broken)
                broken = {
                    **value,
                    "cohorts": {**value["cohorts"], "short:fallback": {"count": 9999}},
                }
                with self.assertRaisesRegex(RuntimeError, "undersampled"):
                    validate_policy_contract_measurement(broken)
                broken = {**value, "per_class_policy": {}}
                with self.assertRaisesRegex(RuntimeError, "class/policy"):
                    validate_policy_contract_measurement(broken)

    def test_reports_refuse_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            write(path, {"status": "failed"})
            with self.assertRaises(FileExistsError):
                write(path, {"status": "passed"})
            self.assertEqual(json.loads(path.read_text()), {"status": "failed"})

    def test_missing_or_ambiguous_wheel_is_not_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "packages").mkdir()
            with self.assertRaises(RuntimeError):
                wheel(root)

    def test_generated_consumer_owns_its_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[package]\nversion="0.3.0"\n', encoding="utf-8"
            )
            (root / "scripts").mkdir()
            (root / "scripts" / "core_consumer.rs").write_text("", encoding="utf-8")
            fixture = root / "fixture"
            fixture.mkdir()
            (fixture / "Cargo.toml").write_text(
                '[package]\npublish=["crates-io"]\n', encoding="utf-8"
            )
            (fixture / ".cargo_vcs_info.json").write_text(
                json.dumps({"git": {"sha1": "a" * 40}}), encoding="utf-8"
            )
            archives = root / "target" / "package"
            archives.mkdir(parents=True)
            with tarfile.open(archives / "normalizer-tr-0.3.0.crate", "w:gz") as tar:
                tar.add(fixture, arcname="normalizer-tr-0.3.0")
            output = root / "target" / "verification"
            output.mkdir()
            with (
                patch("verification.ROOT", root),
                patch("verification.command", return_value="a" * 40),
            ):
                prepare(output)
            manifest = tomllib.loads(
                (output / "rust-consumer" / "Cargo.toml").read_text(encoding="utf-8")
            )
            self.assertEqual(manifest["workspace"], {})


if __name__ == "__main__":
    unittest.main()
