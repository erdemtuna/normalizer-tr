"""Small complete reports; no native work or timing is needed by tooling tests."""

import copy
import hashlib
import unittest
from unittest.mock import patch

from scripts.common import ROOT
from scripts.performance_report import (
    NATIVE_METHOD,
    inventory as full_inventory,
    prepared_inputs,
    python_counts,
)

CASES = [
    {"id": "short-case", "text": "text", "class": "quantity", "cohort": "short"},
    {
        "id": "medium-case",
        "text": "word " * 60,
        "class": "quantity",
        "cohort": "medium",
    },
]
CONTRACT = [{"id": "example", "text": "text", "class": "quantity", "expected": "text"}]


def timing(value=100, count=10000):
    return {
        "count": count,
        "p50_ns": value,
        "p95_ns": value * 2,
        "p99_ns": value * 3,
        "max_ns": value * 4,
    }


def result(text, python=False):
    ranges = (
        {"start_byte": 0, "end_byte": len(text.encode())}
        if python
        else {"range": {"start": 0, "end": len(text.encode())}}
    )
    return {
        "result": {
            "normalized_text": text,
            "locale": "tr-TR",
            "normalizer_id": "old",
            "complete": True,
            "segments": [
                {
                    **ranges,
                    "kind": "verbatim" if python else "Verbatim",
                    "text": text,
                    "rule_id": "text.verbatim",
                }
            ],
            "issues": [],
            "fallbacks": [],
            "fallback_used": False,
        }
    }


def catalog():
    value = copy.deepcopy(full_inventory()[0])
    value["native"] = [
        {
            "suite": "recognition",
            "ids": ["phone"],
            "bytes": [1024],
            "policies": ["preserve"],
        }
    ]
    return value


def section(policies, expanded=False, python=False):
    inputs = prepared_inputs(
        CONTRACT if expanded else CASES, policies, expanded, python
    )
    if not python and not expanded:
        inputs = {
            (key.split(":", 1)[1] if len(policies) == 2 else key.split(":")[1]): value
            for key, value in inputs.items()
        }
    cohorts = (
        {f"{c}:{p}" for c in ("short", "medium") for p in policies}
        if expanded
        else {"short", "medium"}
    )
    counts = python_counts(CONTRACT if expanded else CASES, policies, expanded)
    return {
        "inputs": inputs,
        "snapshots": {
            key: result(value["text"], python) for key, value in inputs.items()
        },
        "order": list(inputs),
        "cohorts": {key: timing() for key in cohorts},
        "per_class_policy": {key: timing(count=count) for key, count in counts.items()},
        "input_cases": copy.deepcopy(CONTRACT),
        "normalizer_id": "old",
        "constructor_first_ns": 100,
        "subsequent_same_process_constructor": timing(count=100),
        "concurrent": {key: timing(count=1000) for key in ("short", "medium")},
    }


def report(value=None):
    value = catalog() if value is None else value
    return {
        **section(("preserve", "reject")),
        "schema_version": 2,
        "report_kind": "native",
        "method_id": value["native_method"],
        "workload_inventory": copy.deepcopy(value),
        "source_sha": "head-A",
        "method": copy.deepcopy(NATIVE_METHOD),
        "normalizer_id": "old",
        "constructor_first_process_init_ns": 100,
        "constructors": timing(count=100),
        "clock_overhead": timing(),
        "distribution": [{"id": "example", "bytes": 4}],
        "fallback_measurement": section(("fallback",)),
        "expanded_coverage_measurement": {
            **section(("preserve", "reject", "fallback"), True),
            "scaling": [],
        },
        "large": [],
        "limit_diagnostics": [],
        "supplementary_scaling": [],
        "reviewed_recognition_scaling": {
            "scaling": [
                {
                    "suite": "recognition",
                    "id": "phone",
                    "bytes": 1024,
                    "policy": "preserve",
                    "method": value["native_long_method"],
                    "timing": timing(count=100),
                    "outcome": result("x" * 1024),
                    "input_fingerprint": {
                        "text": "x" * 1024,
                        "options": {"ambiguity_policy": "Preserve", "hints": []},
                    },
                }
            ],
            "deadline_input_fingerprint": {
                "text": "url",
                "options": {"ambiguity_policy": "Preserve", "hints": []},
            },
            "deadline_ms": 10,
            "electronic_deadline_10ms": [
                {"elapsed_ns": 100, "outcome": result("url")} for _ in range(3)
            ],
        },
    }


def python_report(value):
    return {
        "schema_version": 2,
        "report_kind": "installed-python",
        "method_id": value["python_method"],
        "workload_inventory": copy.deepcopy(value),
        "methods": copy.deepcopy(value["python"]),
        "identity": {
            **{
                key: "sample"
                for key in (
                    "python",
                    "implementation",
                    "executable",
                    "native_path",
                    "build_version",
                    "source_sha",
                    "tooling_sha",
                )
            },
            "import_path": "sample/site-packages/normalizer_tr/__init__.py",
            "normalizer_id": "old",
            "native_sha256": "a" * 64,
        },
        "corpus_sha256": {
            name: hashlib.sha256((ROOT / "benches" / name).read_bytes()).hexdigest()
            for name in ("corpus.json", "intent-corpus.json")
        },
        "policy_contract_sha256": hashlib.sha256(b"").hexdigest(),
        "measurement": section(("preserve", "reject"), python=True),
        "fallback_measurement": section(("fallback",), python=True),
        "expanded_coverage_measurement": section(
            ("preserve", "reject", "fallback"), True, True
        ),
        "clock_overhead": timing(),
    }


class ReportTestCase(unittest.TestCase):
    def setUp(self):
        self.catalog = catalog()
        self.required = {("recognition", "phone", 1024, "preserve")}
        for target, kwargs in (
            ("inventory", {"return_value": (self.catalog, self.required)}),
            ("fixture_cases", {"return_value": copy.deepcopy(CASES)}),
            (
                "policy_contract_fixture",
                {"return_value": (copy.deepcopy(CONTRACT), [])},
            ),
        ):
            stub = patch(f"scripts.performance_report.{target}", **kwargs)
            stub.start()
            self.addCleanup(stub.stop)

    def report(self):
        return report(self.catalog)
