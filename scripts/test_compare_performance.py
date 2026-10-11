import copy
import unittest

from compare_performance import SAMPLES, compare, validate_timing


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
            "cohorts": {"short:preserve": timing()},
        },
        "reviewed_recognition_scaling": {
            "scaling": [
                {
                    "path": "phone",
                    "bytes": 1024,
                    "policy": "preserve",
                    "timing": timing(count=100),
                    "outcome": {"error": "LimitExceeded"},
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


if __name__ == "__main__":
    unittest.main()
