"""Admission validates recorded contracts, not language or grapheme rules."""

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.performance_report import (
    admit_native,
    admit_python,
    inventory,
    load,
    validate_input,
    validate_outcome,
)
from scripts.tests.performance_fixtures import (
    ReportTestCase,
    full_inventory,
    python_report,
    result,
)
from scripts.verification import validate_python_identity


class ReportAdmissionTests(ReportTestCase):
    def test_output_endpoints_cannot_split_utf8_scalars_in_either_schema(self):
        for python in (False, True):

            def bounds(start, end):
                return (
                    {"start_byte": start, "end_byte": end}
                    if python
                    else {"range": {"start": start, "end": end}}
                )

            issue = {
                **bounds(0, 1),
                "category": "ambiguous" if python else "Ambiguous",
                "explanation": "split",
            }
            for kind in ("segments", "issues", "fallbacks", "unresolved"):
                value = result("ö", python)
                record = value["result"]
                if kind == "segments":
                    record["segments"] = [
                        {
                            **bounds(0, 1),
                            "kind": "verbatim" if python else "Verbatim",
                            "text": "ö",
                            "rule_id": "text.verbatim",
                        },
                        {
                            **bounds(1, 2),
                            "kind": "verbatim" if python else "Verbatim",
                            "text": "",
                            "rule_id": "text.verbatim",
                        },
                    ]
                elif kind == "issues":
                    record["issues"] = [issue]
                    record["complete"] = False
                elif kind == "fallbacks":
                    record["fallbacks"] = [
                        {
                            **bounds(1, 2),
                            "attempted_class": "symbol" if python else "Symbol",
                            "reason": "unhandled_symbol"
                            if python
                            else "UnhandledSymbol",
                            "original_category": None,
                            "strategy": "literal" if python else "Literal",
                        }
                    ]
                    record["fallback_used"] = True
                else:
                    value = (
                        {
                            "error": "unresolved",
                            "message": "split",
                            "issues": [issue],
                            "limit_kind": None,
                        }
                        if python
                        else {"error": "Unresolved", "issues": [issue]}
                    )
                with (
                    self.subTest(python=python, kind=kind),
                    self.assertRaisesRegex(RuntimeError, "UTF-8 scalar"),
                ):
                    validate_outcome(value, "ö", "outcome", python)

    def test_valid_multibyte_and_decomposed_output_endpoints_remain_valid(self):
        for python in (False, True):
            for text in ("ö", "o\u0308", "ö 1\r\n"):
                with self.subTest(python=python, text=text):
                    validate_outcome(result(text, python), text, "valid", python)
            bounds = (
                {"start_byte": 0, "end_byte": 3}
                if python
                else {"range": {"start": 0, "end": 3}}
            )
            issue = {
                **bounds,
                "category": "ambiguous" if python else "Ambiguous",
                "explanation": "whole",
            }
            value = (
                {
                    "error": "unresolved",
                    "message": "whole",
                    "issues": [issue],
                    "limit_kind": None,
                }
                if python
                else {"error": "Unresolved", "issues": [issue]}
            )
            validate_outcome(value, "o\u0308", "valid", python)
        fingerprint = {
            "text": "ö",
            "options": {
                "ambiguity_policy": "Preserve",
                "hints": [{"range": {"start": 0, "end": 1}, "kind": "Cardinal"}],
            },
        }
        validate_input(fingerprint, "intentional-invalid-input-hint")
        fingerprint = {
            "text": "ö",
            "options": {
                "ambiguity_policy": "preserve",
                "hints": [{"start_byte": 0, "end_byte": 1, "kind": "cardinal"}],
                "cancellation": None,
                "deadline_ms": None,
            },
        }
        validate_input(
            fingerprint, "intentional-invalid-python-input-hint", python=True
        )

    def test_only_clock_overhead_can_have_zero_quantiles(self):
        value = self.report()
        value["clock_overhead"]["p50_ns"] = 0
        admit_native(value)
        value["reviewed_recognition_scaling"]["scaling"][0]["timing"]["p50_ns"] = 0
        with self.assertRaises(RuntimeError):
            admit_native(value)

    def test_mixed_native_identity_is_not_accepted(self):
        value = self.report()
        value["reviewed_recognition_scaling"]["scaling"][0]["outcome"]["result"][
            "normalizer_id"
        ] = "stale"
        with self.assertRaisesRegex(RuntimeError, "identity mismatch"):
            admit_native(value)

    def test_native_and_python_methods_are_separate(self):
        self.assertEqual(len(admit_native(self.report()).scaling), 1)
        value = python_report(self.catalog)
        self.assertIs(admit_python(value), value)
        with self.assertRaisesRegex(RuntimeError, "wrong report kind"):
            admit_python(self.report())

    def test_python_missing_snapshot_shared_coverage_and_stale_reports_fail(self):
        for change in (
            "version",
            "kind",
            "input",
            "snapshot",
            "class",
            "order",
            "methods",
            "hash",
            "identity",
            "count",
        ):
            value = python_report(self.catalog)
            section = value["measurement"]
            key = next(iter(section["inputs"]))
            if change == "version":
                value["schema_version"] = 1
            elif change == "kind":
                value["report_kind"] = "native"
            elif change == "input":
                section["inputs"].pop(key)
                section["snapshots"].pop(key)
                section["order"].remove(key)
            elif change == "snapshot":
                section["snapshots"][key] = {}
            elif change == "class":
                section["per_class_policy"].clear()
            elif change == "order":
                section["order"].reverse()
            elif change == "methods":
                value["methods"]["concurrent_method"] = "with-disposal"
            elif change == "hash":
                value["policy_contract_sha256"] = "a" * 64
            elif change == "identity":
                value["identity"]["import_path"] = "source/normalizer_tr/__init__.py"
            elif change == "count":
                section["concurrent"]["short"]["count"] = 10000
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                admit_python(value)

    def test_outcome_fields_ranges_and_derived_values_are_required(self):
        for python in (False, True):
            for field in ("complete", "fallback_used", "segments", "normalized_text"):
                value = result("word", python)
                if field in ("complete", "fallback_used"):
                    value["result"][field] = not value["result"][field]
                elif field == "segments":
                    value["result"][field][0]["end_byte" if python else "range"] = (
                        3 if python else {"start": 0, "end": 3}
                    )
                else:
                    value["result"][field] = "another"
                with (
                    self.subTest(python=python, field=field),
                    self.assertRaises(RuntimeError),
                ):
                    validate_outcome(value, "word", "outcome", python)
            for value in (
                {},
                {"result": {}},
                {**result("word", python), "error": "InvalidInput"},
            ):
                with (
                    self.subTest(python=python, value=value),
                    self.assertRaises(RuntimeError),
                ):
                    validate_outcome(value, "word", "outcome", python)

    def test_error_envelopes_cannot_hide_control_or_limit_failures(self):
        validate_outcome(
            {"error": "LimitExceeded", "limit": "Candidates"}, "word", "error"
        )
        validate_outcome({"error": "Cancelled"}, "word", "deadline", deadline=True)
        for value in (
            {"error": "Cancelled"},
            {"error": "Internal"},
            {"error": "InvalidConfiguration"},
            {"error": "LimitExceeded"},
            {"error": "LimitExceeded", "limit": "unknown"},
            {"error": "Unresolved", "issues": []},
        ):
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                validate_outcome(value, "word", "error")
        for code, issues, limit in (
            ("cancelled", [], None),
            ("unresolved", [], None),
            ("invalid_hint", [], "input"),
            ("limit_exceeded", [], None),
        ):
            value = {
                "error": code,
                "message": "failure",
                "issues": issues,
                "limit_kind": limit,
            }
            with self.subTest(code=code), self.assertRaises(RuntimeError):
                validate_outcome(value, "word", "error", True)

    def test_intentional_invalid_hints_are_recorded_not_repaired(self):
        value = {
            "text": "1\r\n1",
            "options": {
                "ambiguity_policy": "Preserve",
                "hints": [{"range": {"start": 0, "end": 2}, "kind": "Cardinal"}],
            },
        }
        self.assertEqual(validate_input(value, "input"), "1\r\n1")
        value["options"]["hints"][0]["range"]["end"] = 999
        validate_input(value, "input")
        value["options"]["hints"][0]["range"]["end"] = True
        with self.assertRaises(RuntimeError):
            validate_input(value, "input")

    def test_installed_identity_matches_consumer_and_revision(self):
        value = python_report(self.catalog)
        identity = value["identity"]
        identity["source_sha"] = identity["tooling_sha"] = "head"
        installed = copy.deepcopy(identity)
        validate_python_identity(value, installed, "head")
        for key in (
            "native_sha256",
            "native_path",
            "python",
            "implementation",
            "executable",
            "import_path",
            "normalizer_id",
            "build_version",
            "source_sha",
            "tooling_sha",
        ):
            changed = copy.deepcopy(value)
            changed["identity"][key] = "stale"
            with self.subTest(key=key), self.assertRaisesRegex(RuntimeError, "differs"):
                validate_python_identity(changed, installed, "head")

    def test_json_duplicate_keys_and_nonfinite_numbers_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            for value in (
                '{"schema_version":2,"schema_version":2}',
                '{"value":NaN}',
                '{"value":Infinity}',
                "{bad}",
            ):
                path.write_text(value)
                with self.subTest(value=value), self.assertRaises(RuntimeError):
                    load(path)

    def test_native_inventory_mutation_cannot_hide_shared_omission(self):
        value = self.report()
        value["workload_inventory"]["native"] = []
        value["reviewed_recognition_scaling"]["scaling"] = []
        with self.assertRaisesRegex(RuntimeError, "inventory differs"):
            admit_native(value)


class InventoryTests(unittest.TestCase):
    def test_reconciled_inventory_is_exact_not_observed_count(self):
        value, keys = full_inventory()
        self.assertEqual(len(keys), 625)
        sizes = {
            suite: sum(key[0] == suite for key in keys)
            for suite in {key[0] for key in keys}
        }
        self.assertEqual(sizes["scanner"], 123)
        self.assertEqual(sizes["source-map"], 132)
        self.assertEqual(sizes["unicode-fallback"], 48)
        self.assertEqual(sizes["resources"], 6)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "benches" / "latency" / "workload_inventory.json"
            path.parent.mkdir(parents=True)
            value["native"].append(copy.deepcopy(value["native"][0]))
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(RuntimeError, "duplicate"):
                inventory(root)
