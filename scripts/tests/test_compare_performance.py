import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.common import ROOT
from scripts.compare_performance import (
    SCALING_SAMPLES,
    SAMPLES,
    compare,
    main,
    validate_timing,
)


def timing(value=100, count=SAMPLES):
    return {
        "count": count,
        "p50_ns": value,
        "p95_ns": value * 2,
        "p99_ns": value * 3,
        "max_ns": value * 4,
    }


def report():
    return {
        "method": {"samples_per_cohort": SAMPLES},
        "cohorts": {"short": timing(), "medium": timing()},
        "distribution": [{"id": "example", "bytes": 4}],
        "snapshots": {
            "example": {"result": {"normalized_text": "spoken", "normalizer_id": "old"}}
        },
        "per_class_policy": {"short:quantity:preserve": timing(count=5000)},
        "fallback_measurement": {
            "snapshots": {"example": {"result": {"normalized_text": "spoken"}}},
            "cohorts": {"short": timing(), "medium": timing()},
        },
        "expanded_coverage_measurement": {
            "input_cases": [{"text": "5 kg", "expected": "spoken"}],
            "cohorts": {
                f"{cohort}:{policy}": timing()
                for cohort in ("short", "medium")
                for policy in ("preserve", "reject", "fallback")
            },
        },
        "reviewed_recognition_scaling": {
            "scaling": [
                {
                    "path": "phone",
                    "bytes": 1024,
                    "policy": "preserve",
                    "timing": timing(count=SCALING_SAMPLES),
                    "outcome": {"error": "LimitExceeded"},
                    "input_fingerprint": {
                        "text": "x" * 1024,
                        "options": {"ambiguity_policy": "Preserve", "hints": []},
                    },
                }
            ]
        },
    }


class PerformanceTests(unittest.TestCase):
    def test_slower_valid_result_is_informational(self):
        after = report()
        after["cohorts"]["short"] = timing(10000)
        self.assertEqual(compare(report(), after, True)["primary"][0]["p95_ratio"], 100)

    def test_changed_work_is_not_a_speedup(self):
        after = report()
        after["snapshots"]["example"]["result"]["normalized_text"] = "different"
        self.assertIsNone(compare(report(), after, True)["primary"][0]["p95_ratio"])
        self.assertIsNone(compare(report(), report(), False)["primary"][0]["p95_ratio"])
        after = report()
        after["expanded_coverage_measurement"]["input_cases"][0]["expected"] = "new"
        self.assertIsNone(
            compare(report(), after, True)["policy_contract"][0]["p95_ratio"]
        )

    def test_invalid_or_incomplete_measurements_fail(self):
        for changes in (
            {"count": 0},
            {"count": SAMPLES - 1},
            {"p95_ns": 0},
            {"p95_ns": 50},
            {"max_ns": float("inf")},
        ):
            with self.subTest(changes=changes), self.assertRaises(RuntimeError):
                validate_timing({**timing(), **changes}, SAMPLES)
        after = copy.deepcopy(report())
        del after["cohorts"]["medium"]
        with self.assertRaises(RuntimeError):
            compare(report(), after, True)
        after = report()
        after["method"] = {}
        with self.assertRaises(RuntimeError):
            compare(report(), after, True)

    def test_scaling_rejects_duplicates_in_either_report(self):
        for duplicate_base in (True, False):
            before, after = report(), report()
            broken = before if duplicate_base else after
            scaling = broken["reviewed_recognition_scaling"]["scaling"]
            scaling.append(copy.deepcopy(scaling[0]))
            with (
                self.subTest(base=duplicate_base),
                self.assertRaisesRegex(RuntimeError, "duplicate recognition"),
            ):
                compare(before, after, True)

    def test_scaling_requires_full_sample_count_even_when_counts_agree(self):
        for count in (1, SCALING_SAMPLES - 1, SCALING_SAMPLES + 1):
            before, after = report(), report()
            for value in (before, after):
                value["reviewed_recognition_scaling"]["scaling"][0]["timing"][
                    "count"
                ] = count
            with (
                self.subTest(count=count),
                self.assertRaisesRegex(RuntimeError, "undersampled"),
            ):
                compare(before, after, True)
        after = report()
        after["reviewed_recognition_scaling"]["scaling"][0]["timing"]["count"] = 1
        with self.assertRaisesRegex(RuntimeError, "undersampled"):
            compare(report(), after, True)

    def test_scaling_identity_covers_equal_length_text_and_options(self):
        valid = compare(report(), report(), True)["recognition_scaling"][0]
        self.assertTrue(valid["comparable"])
        self.assertEqual(valid["base_input_sha256"], valid["head_input_sha256"])
        for change in ("text", "hints"):
            after = report()
            fingerprint = after["reviewed_recognition_scaling"]["scaling"][0][
                "input_fingerprint"
            ]
            if change == "text":
                fingerprint["text"] = "\u00fc" * 512
            else:
                fingerprint["options"]["hints"] = [
                    {"range": {"start": 0, "end": 1}, "kind": "Cardinal"}
                ]
            result = compare(report(), after, True)["recognition_scaling"][0]
            with self.subTest(change=change):
                self.assertFalse(result["comparable"])
                self.assertIsNone(result["p95_ratio"])
                self.assertNotEqual(
                    result["base_input_sha256"], result["head_input_sha256"]
                )

    def test_legacy_scaling_reports_are_readable_but_not_comparable(self):
        for missing in ("base", "head", "both"):
            before, after = report(), report()
            for name, value in (("base", before), ("head", after)):
                if missing in (name, "both"):
                    del value["reviewed_recognition_scaling"]["scaling"][0][
                        "input_fingerprint"
                    ]
            result = compare(before, after, True)
            with self.subTest(missing=missing):
                self.assertTrue(result["primary"][0]["comparable"])
                self.assertFalse(result["recognition_scaling"][0]["comparable"])
                self.assertIsNone(result["recognition_scaling"][0]["p95_ratio"])

    def test_scaling_rejects_invalid_input_identity(self):
        for fingerprint in (
            {},
            {
                "text": "wrong size",
                "options": {"ambiguity_policy": "Preserve", "hints": []},
            },
            {
                "text": "x" * 1024,
                "options": {"ambiguity_policy": "Reject", "hints": []},
            },
        ):
            after = report()
            after["reviewed_recognition_scaling"]["scaling"][0]["input_fingerprint"] = (
                fingerprint
            )
            with (
                self.subTest(fingerprint=fingerprint),
                self.assertRaisesRegex(
                    RuntimeError, "invalid recognition input fingerprint"
                ),
            ):
                compare(report(), after, True)


class PerformanceSourceTests(unittest.TestCase):
    def run_sampler(self, args, **kwargs):
        Path(args[-1]).write_text(json.dumps(report()), encoding="utf-8")

    def test_entry_revisions_are_recorded_and_checked_after_sampling(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
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
                    "scripts.compare_performance.source_revisions",
                    return_value=revisions,
                ) as capture,
                patch("scripts.compare_performance.run_comparison", return_value=[]),
                patch(
                    "scripts.compare_performance.build_sampler",
                    side_effect=["base-bin", "head-bin"],
                ),
                patch(
                    "scripts.compare_performance.subprocess.run",
                    side_effect=self.run_sampler,
                ),
                patch(
                    "scripts.compare_performance.subprocess.check_output",
                    return_value="test compiler",
                ),
                patch(
                    "scripts.compare_performance.corpus_hash",
                    return_value="same inputs",
                ),
                patch("scripts.compare_performance.check_source_revisions") as check,
            ):
                main()
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
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            base, head, output = root / "base", root / "head", root / "report"
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
                    "scripts.compare_performance.source_revisions",
                    return_value={base: "base-A", head: "head-A", ROOT: "tooling-A"},
                ),
                patch("scripts.compare_performance.run_comparison", return_value=[]),
                patch(
                    "scripts.compare_performance.build_sampler",
                    side_effect=["base-bin", "head-bin"],
                ),
                patch(
                    "scripts.compare_performance.subprocess.run",
                    side_effect=self.run_sampler,
                ),
                patch(
                    "scripts.compare_performance.subprocess.check_output",
                    return_value="test compiler",
                ),
                patch(
                    "scripts.compare_performance.corpus_hash",
                    return_value="same inputs",
                ),
                patch(
                    "scripts.compare_performance.check_source_revisions",
                    side_effect=RuntimeError(
                        "source revision changed during comparison"
                    ),
                ),
                self.assertRaisesRegex(RuntimeError, "revision changed"),
            ):
                main()
            self.assertFalse((output / "comparison.json").exists())
            self.assertFalse((output / "summary.md").exists())

    def test_dirty_sources_fail_before_building_or_creating_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report"
            with (
                patch(
                    "sys.argv",
                    ["compare", "--base", directory, "--output", str(output)],
                ),
                patch(
                    "scripts.compare_performance.source_revisions",
                    side_effect=RuntimeError(
                        "comparison requires clean tracked sources"
                    ),
                ),
                patch("scripts.compare_performance.run_comparison") as contracts,
                patch("scripts.compare_performance.build_sampler") as build,
                self.assertRaisesRegex(RuntimeError, "clean tracked"),
            ):
                main()
            contracts.assert_not_called()
            build.assert_not_called()
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
