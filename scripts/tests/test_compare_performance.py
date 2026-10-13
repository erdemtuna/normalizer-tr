"""Checked comparisons are informational only for validated equivalent work."""

import copy
import json
import tempfile
from pathlib import Path
from unittest.mock import patch

from scripts.common import ROOT
from scripts.compare_performance import compare, main, preflight, validate_timing
from scripts.tests.performance_fixtures import ReportTestCase, timing


class PerformanceTests(ReportTestCase):
    def test_shared_invalid_utf8_output_cannot_acquire_a_ratio(self):
        before, after = self.report(), self.report()
        for value in (before, after):
            row = value["reviewed_recognition_scaling"]["scaling"][0]
            row["input_fingerprint"]["text"] = "a" * 1022 + "ö"
            result = row["outcome"]["result"]
            result["segments"] = [
                {
                    "range": {"start": 0, "end": 1023},
                    "kind": "Verbatim",
                    "text": "x" * 1023,
                    "rule_id": "text.verbatim",
                },
                {
                    "range": {"start": 1023, "end": 1024},
                    "kind": "Verbatim",
                    "text": "x",
                    "rule_id": "text.verbatim",
                },
            ]
        with self.assertRaisesRegex(RuntimeError, "UTF-8 scalar"):
            compare(before, after, True)

    def test_expanded_outcome_only_changes_block_policy_cohort_ratios(self):
        for field in ("text", "ranges", "issues", "fallbacks", "medium-rule"):
            before, after = self.report(), self.report()
            snapshots = after["expanded_coverage_measurement"]["snapshots"]
            key = (
                "medium:example:preserve"
                if field == "medium-rule"
                else "short:example:preserve"
            )
            result = snapshots[key]["result"]
            if field == "medium-rule":
                result["segments"][0]["rule_id"] = "changed.medium.rule"
            elif field == "text":
                result["normalized_text"] = "new"
                result["segments"][0]["text"] = "new"
            elif field == "ranges":
                result["segments"] = [
                    {
                        "range": {"start": 0, "end": 2},
                        "kind": "Verbatim",
                        "text": "te",
                        "rule_id": "text.verbatim",
                    },
                    {
                        "range": {"start": 2, "end": 4},
                        "kind": "Verbatim",
                        "text": "xt",
                        "rule_id": "text.verbatim",
                    },
                ]
            elif field == "issues":
                result["issues"] = [
                    {
                        "range": {"start": 0, "end": 4},
                        "category": "Ambiguous",
                        "explanation": "changed",
                    }
                ]
                result["complete"] = False
            else:
                result["fallbacks"] = [
                    {
                        "range": {"start": 0, "end": 4},
                        "attempted_class": "Expression",
                        "reason": "UnsupportedForm",
                        "original_category": None,
                        "strategy": "Literal",
                    }
                ]
                result["fallback_used"] = True
            with self.subTest(field=field):
                compared = compare(before, after, True)
                self.assertTrue(
                    all(
                        not row["comparable"] and row["p95_ratio"] is None
                        for row in compared["policy_contract"]
                    )
                )

    def test_build_identity_only_does_not_change_semantic_work(self):
        before, after = self.report(), self.report()

        def change(value):
            if isinstance(value, dict):
                if "normalizer_id" in value:
                    value["normalizer_id"] = "new"
                for item in value.values():
                    change(item)
            elif isinstance(value, list):
                for item in value:
                    change(item)

        change(after)
        compared = compare(before, after, True)
        self.assertTrue(compared["scaling"][0]["comparable"])
        self.assertTrue(
            all(
                row["comparable"] and row["p95_ratio"] == 1
                for row in compared["policy_contract"]
            )
        )

    def test_diagnostics_ranges_and_rule_changes_block_ratios(self):
        for field in ("issues", "fallbacks", "rule", "partition"):
            before, after = self.report(), self.report()
            result = after["reviewed_recognition_scaling"]["scaling"][0]["outcome"][
                "result"
            ]
            if field == "issues":
                result["issues"] = [
                    {
                        "range": {"start": 0, "end": 1024},
                        "category": "Ambiguous",
                        "explanation": "changed",
                    }
                ]
                result["complete"] = False
            elif field == "fallbacks":
                result["fallbacks"] = [
                    {
                        "range": {"start": 0, "end": 1024},
                        "attempted_class": "Symbol",
                        "reason": "UnhandledSymbol",
                        "original_category": None,
                        "strategy": "Literal",
                    }
                ]
                result["fallback_used"] = True
            elif field == "rule":
                result["segments"][0]["rule_id"] = "changed.rule"
            else:
                result["segments"] = [
                    {
                        "range": {"start": 0, "end": 512},
                        "kind": "Verbatim",
                        "text": "x" * 512,
                        "rule_id": "text.verbatim",
                    },
                    {
                        "range": {"start": 512, "end": 1024},
                        "kind": "Verbatim",
                        "text": "x" * 512,
                        "rule_id": "text.verbatim",
                    },
                ]
            with self.subTest(field=field):
                row = compare(before, after, True)["scaling"][0]
                self.assertIsNone(row["p95_ratio"])
                self.assertIn("semantic outcome changed", row["reason"])

    def test_slower_valid_result_is_informational(self):
        after = self.report()
        after["cohorts"]["short"] = timing(10000)
        self.assertEqual(
            next(
                row
                for row in compare(self.report(), after, True)["primary"]
                if row["cohort"] == "short"
            )["p95_ratio"],
            100,
        )

    def test_changed_work_is_not_a_speedup(self):
        after = self.report()
        after["snapshots"]["short-case:preserve"]["result"]["normalized_text"] = (
            "changed"
        )
        after["snapshots"]["short-case:preserve"]["result"]["segments"][0]["text"] = (
            "changed"
        )
        self.assertIsNone(
            compare(self.report(), after, True)["primary"][0]["p95_ratio"]
        )
        self.assertIsNone(
            compare(self.report(), self.report(), False)["primary"][0]["p95_ratio"]
        )
        after = self.report()
        after["expanded_coverage_measurement"]["input_cases"][0]["expected"] = (
            "changed golden"
        )
        self.assertIsNone(
            compare(self.report(), after, True)["policy_contract"][0]["p95_ratio"]
        )
        after = self.report()
        after["method"] = {}
        with self.assertRaisesRegex(RuntimeError, "methods differ"):
            compare(self.report(), after, True)

    def test_invalid_timing_fails(self):
        for changes in (
            {"count": 0},
            {"count": 9999},
            {"p95_ns": 0},
            {"p95_ns": 50},
            {"max_ns": float("inf")},
            {"count": True},
        ):
            with self.subTest(changes=changes), self.assertRaises(RuntimeError):
                validate_timing({**timing(), **changes}, 10000)

    def test_duplicates_missing_and_shared_omissions_fail(self):
        for location in ("base", "head", "both"):
            for duplicate in (False, True):
                before, after = self.report(), self.report()
                for name, value in (("base", before), ("head", after)):
                    if location in (name, "both"):
                        rows = value["reviewed_recognition_scaling"]["scaling"]
                        rows.append(
                            copy.deepcopy(rows[0])
                        ) if duplicate else rows.clear()
                with (
                    self.subTest(location=location, duplicate=duplicate),
                    self.assertRaises(RuntimeError),
                ):
                    compare(before, after, True)

    def test_identity_and_changed_limit_kind_are_not_comparable(self):
        before, after = self.report(), self.report()
        after["reviewed_recognition_scaling"]["scaling"][0]["input_fingerprint"][
            "text"
        ] = "z" * 1024
        row = compare(before, after, True)["scaling"][0]
        self.assertIsNone(row["p95_ratio"])
        self.assertEqual(row["reason"], "input/options changed")
        before, after = self.report(), self.report()
        before["reviewed_recognition_scaling"]["scaling"][0]["outcome"] = {
            "error": "LimitExceeded",
            "limit": "Candidates",
        }
        after["reviewed_recognition_scaling"]["scaling"][0]["outcome"] = {
            "error": "LimitExceeded",
            "limit": "Result",
        }
        self.assertIsNone(compare(before, after, True)["scaling"][0]["p95_ratio"])

    def test_empty_outcomes_old_versions_and_absent_identity_fail(self):
        for change in ("outcome", "input", "version", "count"):
            after = self.report()
            row = after["reviewed_recognition_scaling"]["scaling"][0]
            if change == "outcome":
                row["outcome"] = {}
            if change == "input":
                del row["input_fingerprint"]
            if change == "version":
                after["schema_version"] = 1
            if change == "count":
                row["timing"]["count"] = 99
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                compare(self.report(), after, True)

    def test_every_long_section_is_compared(self):
        for section, suite in (
            ("large", "large"),
            ("limit_diagnostics", "limits"),
            ("supplementary_scaling", "scanner"),
        ):
            self.catalog["native"][0]["suite"] = suite
            self.required.clear()
            self.required.add((suite, "phone", 1024, "preserve"))
            before, after = self.report(), self.report()
            for value in (before, after):
                value[section] = value["reviewed_recognition_scaling"]["scaling"]
                value[section][0]["suite"] = suite
                value["reviewed_recognition_scaling"]["scaling"] = []
            self.assertEqual(compare(before, after, True)["scaling"][0]["p95_ratio"], 1)


class PerformanceSourceTests(ReportTestCase):
    def run_sampler(self, args, **kwargs):
        value = self.report()
        value["source_sha"] = "base-A" if args[0] == "base-bin" else "head-A"
        Path(args[-1]).write_text(json.dumps(value), encoding="utf-8")

    def run_main(self, root, drift=False):
        root = root.resolve()
        base, head, output = root / "base", root / "head", root / "report"
        revisions = {base: "base-A", head: "head-A", ROOT: "tooling-A"}
        with (
            patch(
                "sys.argv",
                [
                    "compare",
                    "--base",
                    str(base),
                    "--head",
                    str(head),
                    "--output",
                    str(output),
                ],
            ),
            patch(
                "scripts.compare_performance.source_revisions", return_value=revisions
            ) as capture,
            patch("scripts.compare_performance.preflight", return_value=self.catalog),
            patch("scripts.compare_performance.run_comparison", return_value=[]),
            patch(
                "scripts.compare_performance.build_sampler",
                side_effect=["base-bin", "head-bin"],
            ),
            patch(
                "scripts.compare_performance.subprocess.run",
                side_effect=self.run_sampler,
            ) as sampler,
            patch(
                "scripts.compare_performance.subprocess.check_output",
                return_value="compiler",
            ),
            patch("scripts.compare_performance.corpus_hash", return_value="corpus"),
            patch(
                "scripts.compare_performance.check_source_revisions",
                side_effect=RuntimeError("revision changed") if drift else None,
            ) as check,
        ):
            main()
        capture.assert_called_once_with(base, head, ROOT)
        check.assert_called_once_with(revisions)
        self.assertEqual(
            [call.args[0][0] for call in sampler.call_args_list],
            ["base-bin", "head-bin", "head-bin", "base-bin", "base-bin", "head-bin"],
        )
        return output

    def test_revision_capture_and_serial_alternating_order(self):
        with tempfile.TemporaryDirectory() as directory:
            output = self.run_main(Path(directory))
            value = json.loads((output / "comparison.json").read_text())
            self.assertEqual(value["base_sha"], "base-A")
            self.assertEqual(value["tooling_sha"], "tooling-A")
            self.assertEqual(value["schema_version"], 2)
            self.assertIn("recognition", (output / "summary.md").read_text())

    def test_source_drift_cannot_publish_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(RuntimeError, "revision changed"):
                self.run_main(root, True)
            self.assertFalse((root / "report" / "comparison.json").exists())

    def test_old_baseline_fails_before_building(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, "workload_inventory"):
                preflight(Path(directory), ROOT)

    def test_dirty_sources_fail_before_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report"
            with (
                patch(
                    "sys.argv",
                    ["compare", "--base", directory, "--output", str(output)],
                ),
                patch(
                    "scripts.compare_performance.source_revisions",
                    side_effect=RuntimeError("clean tracked sources"),
                ),
                patch("scripts.compare_performance.build_sampler") as build,
                self.assertRaisesRegex(RuntimeError, "clean tracked"),
            ):
                main()
            build.assert_not_called()
            self.assertFalse(output.exists())

    def test_build_failure_cannot_publish_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / "report"
            with (
                patch(
                    "sys.argv",
                    ["compare", "--base", str(root), "--output", str(output)],
                ),
                patch(
                    "scripts.compare_performance.source_revisions",
                    return_value={root: "base", ROOT: "head"},
                ),
                patch(
                    "scripts.compare_performance.preflight", return_value=self.catalog
                ),
                patch("scripts.compare_performance.run_comparison", return_value=[]),
                patch(
                    "scripts.compare_performance.build_sampler",
                    side_effect=RuntimeError("build failed"),
                ),
                patch("scripts.compare_performance.corpus_hash", return_value="corpus"),
                self.assertRaisesRegex(RuntimeError, "build failed"),
            ):
                main()
            self.assertFalse((output / "comparison.json").exists())
            self.assertFalse((output / "summary.md").exists())
