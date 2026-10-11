"""The behavior report must expose differences without inventing approvals."""

import copy
import unittest

from scripts.compare_contracts import compare


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


if __name__ == "__main__":
    unittest.main()
