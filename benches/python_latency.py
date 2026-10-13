"""Dev-only installed-wheel latency sampler; no model imports or network."""

import argparse
import concurrent.futures
import dataclasses
import hashlib
import importlib
import json
import platform
import sys
import time
from pathlib import Path

from scripts.common import ROOT, check_source_revisions, source_revisions, write
from scripts.performance_report import (
    VERSION,
    admit_python,
    inventory,
)


def binding():
    global Normalizer, Hint, NormalizationError
    from normalizer_tr import Hint, NormalizationError, Normalizer

    return importlib.import_module("normalizer_tr")


def fingerprint(text, options):
    return {
        "text": text,
        "options": {
            "ambiguity_policy": options["ambiguity_policy"],
            "hints": [dataclasses.asdict(hint) for hint in options.get("hints", ())],
            "cancellation": None,
            "deadline_ms": None,
        },
    }


def snapshot(n, text, options):
    try:
        result = n.normalize(text, **options)
    except NormalizationError as error:
        return {
            "error": error.code,
            "message": str(error),
            "issues": [dataclasses.asdict(issue) for issue in error.issues],
            "limit_kind": error.limit_kind,
        }
    value = dataclasses.asdict(result)
    for field in ("segments", "issues", "fallbacks"):
        value[field] = list(value[field])
    return {"result": {**value, "fallback_used": result.fallback_used}}


def quantiles(values):
    values = sorted(values)

    def at(percent):
        return values[max(0, (len(values) * percent + 99) // 100 - 1)]

    return {
        "count": len(values),
        "p50_ns": at(50),
        "p95_ns": at(95),
        "p99_ns": at(99),
        "max_ns": values[-1],
    }


def invoke(n, text, options):
    try:
        return n.normalize(text, **options)
    except NormalizationError as error:
        return (error.code, error.issues, error.limit_kind)


def measure(corpus, policies=("preserve", "reject")):
    binding()
    begin = time.perf_counter_ns()
    n = Normalizer()
    first = time.perf_counter_ns() - begin
    constructors = []
    for _ in range(100):
        begin = time.perf_counter_ns()
        instance = Normalizer()
        del instance
        constructors.append(time.perf_counter_ns() - begin)
    cohorts = {}
    per_class = {}
    concurrent_timings = {}
    inputs = {}
    snapshots = {}
    order = []
    for cohort in ("short", "medium"):
        prepared = []
        for case in corpus:
            if case["cohort"] != cohort:
                continue
            for policy in policies:
                options = {"ambiguity_policy": policy}
                if "hint" in case:
                    h = case["hint"]
                    options["hints"] = (Hint(h["start"], h["end"], h["kind"]),)
                expected = invoke(n, case["text"], options)
                key = f"{cohort}:{case['id']}:{policy}"
                inputs[key] = fingerprint(case["text"], options)
                snapshots[key] = snapshot(n, case["text"], options)
                order.append(key)
                prepared.append((case, options, expected, policy))
        for index in range(2000):
            case, options, _, _ = prepared[index % len(prepared)]
            invoke(n, case["text"], options)
        times = []
        for index in range(10000):
            case, options, expected, policy = prepared[index % len(prepared)]
            if index < len(prepared):
                assert invoke(n, case["text"], options) == expected
            start = time.perf_counter_ns()
            result = invoke(n, case["text"], options)
            del result
            elapsed = time.perf_counter_ns() - start
            times.append(elapsed)
            per_class.setdefault(f"{cohort}:{case['class']}:{policy}", []).append(
                elapsed
            )
        for case, options, expected, policy in prepared:
            assert invoke(n, case["text"], options) == expected
            key = f"{cohort}:{case['id']}:{policy}"
            assert snapshot(n, case["text"], options) == snapshots[key]
        cohorts[cohort] = quantiles(times)

        def worker(index):
            case, options, expected, _ = prepared[index % len(prepared)]
            start = time.perf_counter_ns()
            result = invoke(n, case["text"], options)
            elapsed = time.perf_counter_ns() - start
            assert result == expected
            return elapsed

        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            samples = list(pool.map(worker, range(1000)))
        concurrent_timings[cohort] = quantiles(samples)
        for case, options, _, policy in prepared:
            key = f"{cohort}:{case['id']}:{policy}"
            assert snapshot(n, case["text"], options) == snapshots[key]
    return {
        "inputs": inputs,
        "snapshots": snapshots,
        "order": order,
        "normalizer_id": n.normalizer_id,
        "constructor_first_ns": first,
        "subsequent_same_process_constructor": quantiles(constructors),
        "cohorts": cohorts,
        "per_class_policy": {k: quantiles(v) for k, v in per_class.items()},
        "concurrent": concurrent_timings,
    }


def policy_contract_measurement():
    binding()
    path = (
        Path(__file__).resolve().parents[1]
        / "tests"
        / "fixtures"
        / "policy-contract.json"
    )
    files = [path] + [
        path.parent / name for name in json.loads(path.read_text(encoding="utf-8"))
    ]
    cases = [
        case
        for file in files[1:]
        for case in json.loads(file.read_text(encoding="utf-8"))
    ]
    n = Normalizer()
    cohorts = {}
    per_class = {}
    inputs = {}
    snapshots = {}
    order = []
    for cohort in ("short", "medium"):
        prefix = (
            (
                "Bu sentetik paragraf kaynak metnin ve miktarların birlikte okunmasını kontrol eder. "
                * 4
            )
            if cohort == "medium"
            else ""
        )
        for policy in ("preserve", "reject", "fallback"):
            prepared = []
            for case in cases:
                text = prefix + case["text"]
                assert (
                    len(text.encode("utf-8")) <= 256
                    if cohort == "short"
                    else 257 <= len(text.encode("utf-8")) <= 1024
                )
                options = {"ambiguity_policy": policy}
                if "hint" in case:
                    hint = case["hint"]
                    shift = len(prefix.encode("utf-8"))
                    options["hints"] = (
                        Hint(hint["start"] + shift, hint["end"] + shift, hint["kind"]),
                    )
                expected = invoke(n, text, options)
                key = f"{cohort}:{case['id']}:{policy}"
                inputs[key] = fingerprint(text, options)
                snapshots[key] = snapshot(n, text, options)
                order.append(key)
                unresolved = "category" in case
                if unresolved and policy == "reject":
                    assert isinstance(expected, tuple) and expected[0] == "unresolved"
                else:
                    written = (
                        case["fallback"]
                        if unresolved and policy == "fallback"
                        else case["text"]
                        if unresolved
                        else case["expected"]
                    )
                    assert expected.normalized_text == prefix + written
                    assert expected.complete == (not unresolved or policy == "fallback")
                    assert expected.fallback_used == (
                        unresolved and policy == "fallback"
                    )
                prepared.append((case, text, options, expected))
            for index in range(2000):
                _, text, options, _ = prepared[index % len(prepared)]
                invoke(n, text, options)
            samples = []
            for index in range(10000):
                case, text, options, _ = prepared[index % len(prepared)]
                start = time.perf_counter_ns()
                result = invoke(n, text, options)
                del result
                elapsed = time.perf_counter_ns() - start
                samples.append(elapsed)
                per_class.setdefault(f"{cohort}:{case['class']}:{policy}", []).append(
                    elapsed
                )
            for case, text, options, expected in prepared:
                assert invoke(n, text, options) == expected
                key = f"{cohort}:{case['id']}:{policy}"
                assert snapshot(n, text, options) == snapshots[key]
            cohorts[f"{cohort}:{policy}"] = quantiles(samples)
    return {
        "inputs": inputs,
        "snapshots": snapshots,
        "order": order,
        "input_sha256": hashlib.sha256(
            b"".join(file.read_bytes() for file in files)
        ).hexdigest(),
        "input_cases": cases,
        "method": "separate policy cohorts; reviewed goldens; 2000 warmups and 10000 installed public calls including disposal; no filtering",
        "cohorts": cohorts,
        "per_class_policy": {
            key: quantiles(values) for key, values in per_class.items()
        },
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    revisions = source_revisions(ROOT)
    module = binding()
    native_path = Path(module._native.__file__).resolve()
    native_hash = hashlib.sha256(native_path.read_bytes()).hexdigest()
    if "site-packages" not in module.__file__:
        raise RuntimeError(
            "sampler requires the installed wheel, not the source facade"
        )
    catalog, _ = inventory()
    files = [
        Path(__file__).with_name(name) for name in ("corpus.json", "intent-corpus.json")
    ]
    corpus = [
        case for file in files for case in json.loads(file.read_text(encoding="utf-8"))
    ]
    clock = []
    for _ in range(10000):
        start = time.perf_counter_ns()
        clock.append(time.perf_counter_ns() - start)
    report = {
        "schema_version": VERSION,
        "report_kind": "installed-python",
        "method_id": catalog["python_method"],
        "workload_inventory": catalog,
        "methods": catalog["python"],
        "identity": {
            "python": platform.python_version(),
            "implementation": platform.python_implementation(),
            "executable": str(Path(sys.executable).resolve()),
            "native_path": str(native_path),
            "native_sha256": native_hash,
            "import_path": str(Path(module.__file__).resolve()),
            "build_version": module.BUILD_VERSION,
            "normalizer_id": module.NORMALIZER_ID,
            "source_sha": revisions[ROOT],
            "tooling_sha": revisions[ROOT],
        },
        "corpus_sha256": {
            file.name: hashlib.sha256(file.read_bytes()).hexdigest() for file in files
        },
        "clock_overhead": quantiles(clock),
        "measurement": measure(corpus),
        "fallback_measurement": measure(corpus, ("fallback",)),
        "expanded_coverage_measurement": policy_contract_measurement(),
        "allocation_instrumentation": "unavailable for Rust native allocations; no inferred counts or peak-memory claim",
    }
    report["policy_contract_sha256"] = report["expanded_coverage_measurement"][
        "input_sha256"
    ]
    check_source_revisions(revisions)
    if hashlib.sha256(native_path.read_bytes()).hexdigest() != native_hash:
        raise RuntimeError("loaded native file changed during sampling")
    admit_python(report)
    write(args.output, report)


if __name__ == "__main__":
    main()
