import copy

from scripts.performance_comparison import compare
from scripts.performance_summary import render_summary
from scripts.tests.performance_fixtures import ReportTestCase


class SummaryTests(ReportTestCase):
    def comparison(self):
        before = self.report()
        pair = compare(before, copy.deepcopy(before), True)
        return {
            "comparison_basis": {
                "requested_base_sha": "base-A",
                "measurement_base_sha": "base-A",
                "explanation": "Measurements use the requested base revision.",
            },
            "head_sha": "head-A",
            "repetitions": [pair],
        }

    def test_rendering_is_deterministic_and_does_not_change_evidence(self):
        comparison = self.comparison()
        original = copy.deepcopy(comparison)
        rendered = render_summary(comparison)
        self.assertEqual(rendered, render_summary(comparison))
        self.assertEqual(comparison, original)
        self.assertIn("Requested base: `base-A`", rendered)
        self.assertIn("head: `head-A`", rendered)
        self.assertIn("result (complete)", rendered)
        self.assertIn("All checked long-input comparisons", rendered)

    def test_changed_work_is_visible_but_not_ranked_by_a_ratio(self):
        comparison = self.comparison()
        pair = comparison["repetitions"][0]
        pair["primary"][0].update(comparable=False, p95_ratio=None)
        pair["scaling"][0].update(comparable=False, p95_ratio=None)
        rendered = render_summary(comparison)
        self.assertIn("non-comparable", rendered)
        self.assertIn("| recognition | 1 | 1 |", rendered)
        self.assertNotIn("recognition / phone", rendered)
