"""The behavior report must expose differences without inventing approvals."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.common import ROOT
from scripts.compare_contracts import compare, run_comparison


def rows():
    return [
        {
            "id": "quantity",
            "policy": policy,
            "outcome": {
                "result": {
                    "normalizer_id": "normalizer-tr/0.7.0",
                    "normalized_text": "spoken",
                    "complete": True,
                    "segments": [{"range": {"start": 0, "end": 4}}],
                    "issues": [],
                }
            },
        }
        for policy in ("preserve", "reject", "fallback")
    ]


class ComparisonTests(unittest.TestCase):
    def test_identity_is_provenance_not_changed_behavior(self):
        before = rows()
        after = copy.deepcopy(before)
        after[0]["outcome"]["result"]["normalizer_id"] = "normalizer-tr/0.8.0"
        self.assertEqual(compare(before, after), [])
        self.assertIn("normalizer_id", before[0]["outcome"]["result"])

    def test_text_range_and_diagnostic_changes_are_visible(self):
        for field, value in (
            ("normalized_text", "different"),
            ("segments", [{"range": {"start": 0, "end": 3}}]),
            ("issues", [{"category": "Ambiguous"}]),
            ("complete", False),
        ):
            with self.subTest(field=field):
                after = rows()
                after[0]["outcome"]["result"][field] = value
                change = compare(rows(), after)
                self.assertEqual(len(change), 1)
                self.assertEqual(change[0]["policy"], "preserve")
                self.assertEqual(change[0]["after"]["result"][field], value)

    def test_structured_errors_are_compared(self):
        after = rows()
        after[1]["outcome"] = {"error": "LimitExceeded", "limit": "Candidates"}
        self.assertEqual(compare(rows(), after)[0]["after"], after[1]["outcome"])

    def test_missing_duplicate_or_unknown_policies_fail(self):
        for broken in ([], rows()[:2], rows() + rows()[:1]):
            with self.subTest(rows=broken), self.assertRaises(RuntimeError):
                compare(rows(), broken)
        broken = rows()
        broken[0]["policy"] = "unknown"
        with self.assertRaises(RuntimeError):
            compare(rows(), broken)
        broken = rows()
        for row in broken:
            row["id"] = "different-input"
        with self.assertRaisesRegex(RuntimeError, "same cases"):
            compare(rows(), broken)

    def test_requested_coverage_rejects_shared_omissions_and_extra_cases(self):
        with self.assertRaisesRegex(RuntimeError, "coverage differs"):
            compare(rows(), rows(), {"quantity", "omitted"})
        with self.assertRaisesRegex(RuntimeError, "coverage differs"):
            compare(rows(), rows(), {"different"})
        complete = rows()
        for row in rows():
            row["id"] = "second"
            complete.append(row)
        self.assertEqual(compare(complete, complete, {"quantity", "second"}), [])


class ComparisonSourceTests(unittest.TestCase):
    def test_entry_revisions_are_recorded_and_checked_before_success(self):
        base, head = Path("base"), Path("head")
        revisions = {base: "base-A", head: "head-A", ROOT: "tooling-A"}
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report"
            with (
                patch(
                    "scripts.compare_contracts.source_revisions", return_value=revisions
                ) as capture,
                patch(
                    "scripts.compare_contracts.inputs",
                    return_value=[{"id": "quantity", "text": "5 kg"}],
                ),
                patch("scripts.compare_contracts.probe", side_effect=[rows(), rows()]),
                patch("scripts.compare_contracts.check_source_revisions") as check,
            ):
                run_comparison(base, head, output)
            capture.assert_called_once_with(base, head, ROOT)
            check.assert_called_once_with(revisions)
            result = json.loads((output / "comparison.json").read_text())
            self.assertEqual(
                {key: result[key] for key in ("base_sha", "head_sha", "tooling_sha")},
                {
                    "base_sha": "base-A",
                    "head_sha": "head-A",
                    "tooling_sha": "tooling-A",
                },
            )

    def test_source_drift_prevents_success_report(self):
        base, head = Path("base"), Path("head")
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report"
            with (
                patch(
                    "scripts.compare_contracts.source_revisions",
                    return_value={base: "base-A", head: "head-A", ROOT: "tooling-A"},
                ),
                patch(
                    "scripts.compare_contracts.inputs",
                    return_value=[{"id": "quantity", "text": "5 kg"}],
                ),
                patch("scripts.compare_contracts.probe", side_effect=[rows(), rows()]),
                patch(
                    "scripts.compare_contracts.check_source_revisions",
                    side_effect=RuntimeError(
                        "source revision changed during comparison"
                    ),
                ),
                self.assertRaisesRegex(RuntimeError, "revision changed"),
            ):
                run_comparison(base, head, output)
            self.assertFalse((output / "comparison.json").exists())
            self.assertFalse((output / "summary.md").exists())

    def test_dirty_sources_fail_before_probing_or_creating_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report"
            with (
                patch(
                    "scripts.compare_contracts.source_revisions",
                    side_effect=RuntimeError(
                        "comparison requires clean tracked sources"
                    ),
                ),
                patch("scripts.compare_contracts.probe") as probe,
                self.assertRaisesRegex(RuntimeError, "clean tracked"),
            ):
                run_comparison(Path("base"), Path("head"), output)
            probe.assert_not_called()
            self.assertFalse(output.exists())

    def test_mutually_incomplete_probes_cannot_report_full_requested_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report"
            with (
                patch("scripts.compare_contracts.source_revisions", return_value={}),
                patch(
                    "scripts.compare_contracts.inputs",
                    return_value=[{"id": "quantity"}, {"id": "omitted"}],
                ),
                patch("scripts.compare_contracts.probe", side_effect=[rows(), rows()]),
                self.assertRaisesRegex(RuntimeError, "coverage differs"),
            ):
                run_comparison(Path("base"), Path("head"), output)
            self.assertFalse((output / "comparison.json").exists())


if __name__ == "__main__":
    unittest.main()
