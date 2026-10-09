"""Dev-only installed-wheel latency sampler; no model imports or network."""

import argparse
import concurrent.futures
import hashlib
import json
import platform
import time
from pathlib import Path

from normalizer_tr import (
    Normalizer,
    Hint,
    NormalizationError,
    _native,
)


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
        for case, options, expected, _ in prepared:
            assert invoke(n, case["text"], options) == expected
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
    return {
        "normalizer_id": n.normalizer_id,
        "constructor_first_ns": first,
        "subsequent_same_process_constructor": quantiles(constructors),
        "cohorts": cohorts,
        "per_class_policy": {k: quantiles(v) for k, v in per_class.items()},
        "eight_thread_public_call_including_gil_wait_not_executor_queue": concurrent_timings,
    }


def policy_contract_measurement():
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
            for _, text, options, expected in prepared:
                assert invoke(n, text, options) == expected
            cohorts[f"{cohort}:{policy}"] = quantiles(samples)
    return {
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
        "schema_version": 1,
        "python": platform.python_version(),
        "native_path": _native.__file__,
        "native_sha256": hashlib.sha256(
            Path(_native.__file__).read_bytes()
        ).hexdigest(),
        "corpus_sha256": {
            file.name: hashlib.sha256(file.read_bytes()).hexdigest() for file in files
        },
        "method": "10000 public calls/cohort after2000warmup; argument validation native Rust marshal/disposal included; deterministic preserve/reject; no outlier removal/overhead subtraction",
        "clock_overhead": quantiles(clock),
        "measurement": measure(corpus),
        "fallback_measurement": measure(corpus, ("fallback",)),
        "expanded_coverage_measurement": policy_contract_measurement(),
        "fallback_method": "separate fallback-only cohorts, same corpus; 10000 calls/cohort after 2000 warmup; no filtering or overhead subtraction; not equivalent to preserve/reject",
        "allocation_instrumentation": "unavailable for Rust native allocations; no inferred counts or peak-memory claim",
    }
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(report, stream, ensure_ascii=False, indent=2)


if __name__ == "__main__":
    main()
