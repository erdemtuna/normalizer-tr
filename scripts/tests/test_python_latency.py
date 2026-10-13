"""Bounded installed facade serialization checks, never latency sampling."""

import ast
import dataclasses
import inspect
import types
import unittest
from unittest.mock import patch

from benches import python_latency
from scripts.performance_report import validate_input, validate_outcome


@dataclasses.dataclass(frozen=True)
class Hint:
    start_byte: int
    end_byte: int
    kind: str


@dataclasses.dataclass(frozen=True)
class Segment:
    start_byte: int
    end_byte: int
    kind: str
    text: str
    rule_id: str


@dataclasses.dataclass(frozen=True)
class Result:
    normalized_text: str
    locale: str = "tr-TR"
    normalizer_id: str = "old"
    complete: bool = True
    segments: tuple = ()
    issues: tuple = ()
    fallbacks: tuple = ()

    @property
    def fallback_used(self):
        return bool(self.fallbacks)


class Error(Exception):
    code = "limit_exceeded"
    issues = ()
    limit_kind = "result"


class PythonLatencyTests(unittest.TestCase):
    def setUp(self):
        module = types.SimpleNamespace(
            Normalizer=object, Hint=Hint, NormalizationError=Error
        )
        stub = patch.dict("sys.modules", {"normalizer_tr": module})
        stub.start()
        self.addCleanup(stub.stop)
        python_latency.binding()

    def test_success_snapshot_includes_derived_fields_and_lists(self):
        normalizer = types.SimpleNamespace(
            normalize=lambda *args, **kwargs: Result(
                "ö", segments=(Segment(0, 2, "verbatim", "ö", "text.verbatim"),)
            )
        )
        outcome = python_latency.snapshot(
            normalizer, "ö", {"ambiguity_policy": "preserve"}
        )
        self.assertIsInstance(outcome["result"]["segments"], list)
        self.assertFalse(outcome["result"]["fallback_used"])
        validate_outcome(outcome, "ö", "snapshot", True)
        fingerprint = python_latency.fingerprint(
            "ö", {"ambiguity_policy": "preserve", "hints": (Hint(0, 3, "cardinal"),)}
        )
        validate_input(fingerprint, "fingerprint", python=True)
        self.assertEqual(fingerprint["options"]["hints"][0]["end_byte"], 3)

    def test_error_snapshot_preserves_message_without_changing_timed_wrapper(self):
        def normalize(*args, **kwargs):
            raise Error("real failure")

        normalizer = types.SimpleNamespace(normalize=normalize)
        self.assertEqual(
            python_latency.invoke(normalizer, "1", {}), ("limit_exceeded", (), "result")
        )
        snapshot = python_latency.snapshot(normalizer, "1", {})
        self.assertEqual(snapshot["message"], "real failure")
        validate_outcome(snapshot, "1", "snapshot", True)
        timed = ast.parse(inspect.getsource(python_latency.invoke))
        self.assertNotIn("snapshot", inspect.getsource(python_latency.invoke))
        self.assertFalse(
            any(
                isinstance(node, ast.Call)
                and isinstance(node.func, ast.Attribute)
                and node.func.attr in ("asdict", "dumps")
                for node in ast.walk(timed)
            )
        )

    def test_invalid_setup_exceptions_are_not_defaulted(self):
        def normalize(*args, **kwargs):
            raise ValueError("invalid setup")

        normalizer = types.SimpleNamespace(normalize=normalize)
        for function in (python_latency.invoke, python_latency.snapshot):
            with self.assertRaisesRegex(ValueError, "invalid setup"):
                function(normalizer, "1", {})

    def test_quantiles_preserve_all_outliers(self):
        value = python_latency.quantiles(list(range(1, 100)) + [10000])
        self.assertEqual(value["count"], 100)
        self.assertEqual(value["p95_ns"], 95)
        self.assertEqual(value["max_ns"], 10000)
