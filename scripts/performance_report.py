"""Strict, development-only native and installed-Python report admission."""

import hashlib
import json
import re
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

from .common import ROOT, policy_contract_fixture

VERSION = 2
INVENTORY_PATH = Path("benches") / "latency" / "workload_inventory.json"
POLICIES = ("preserve", "reject", "fallback")
QUANTILES = ("p50_ns", "p95_ns", "p99_ns", "max_ns")
NATIVE_METHOD = {
    "samples_per_cohort": 10000,
    "warmup_per_cohort": 2000,
    "timing": "Instant per call; validation mapping recognition rendering owned result disposal included",
    "order": "deterministic corpus order alternating preserve/reject",
    "outliers": "all retained; no overhead subtraction",
}
PYTHON_METHOD = {
    "sequential_samples": 10000,
    "warmup_calls": 2000,
    "constructor_samples": 100,
    "concurrent_samples": 1000,
    "workers": 8,
    "sequential_method": "public-invoke-error-conversion-disposal",
    "concurrent_method": "public-invoke-gil-wait-no-executor-queue-no-disposal",
}
PREFIX = (
    "Bu sentetik paragraf kaynak metnin ve miktarların birlikte okunmasını kontrol eder. "
    * 4
)
KINDS = (
    "Verbatim Unresolved Cardinal Ordinal Decimal Digits Percent Money Unit Date Time "
    "Abbreviation Range Telephone Iban Roman Electronic Symbol Fallback Pronunciation"
).split()
HINT_KINDS = (
    "Cardinal Digits Date Time Ordinal Roman Range Telephone Electronic".split()
)
CATEGORIES = (
    "Ambiguous InvalidExpression ProtectedIdentifier Unsupported UnknownAbbreviation"
).split()
CLASSES = (
    "Number Date Time Percent Quantity Abbreviation Identifier Roman Electronic "
    "Expression Symbol Pronunciation"
).split()
REASONS = (
    "MissingIntent LeadingZeroes InvalidForm ProtectedIdentifier UnsupportedForm "
    "UnapprovedAbbreviation UnhandledSymbol"
).split()
STRATEGIES = (
    "PreferredNumber PreferredDate PreferredTime SurfaceDate SurfaceTime Literal "
    "UnicodeCodePoint"
).split()
PYTHON_LABELS = {
    tuple(values): tuple(
        re.sub(r"(?<!^)(?=[A-Z])", "_", value).lower() for value in values
    )
    for values in (KINDS, HINT_KINDS, CATEGORIES, CLASSES, REASONS, STRATEGIES)
}


def fail(path, message):
    raise RuntimeError(f"{path}: {message}")


def pairs(items):
    value = {}
    for key, item in items:
        if key in value:
            fail("JSON", f"duplicate object key {key!r}")
        value[key] = item
    return value


def load(path):
    try:
        return json.loads(
            Path(path).read_text(encoding="utf-8"),
            object_pairs_hook=pairs,
            parse_constant=lambda value: fail("JSON", f"non-finite number {value}"),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(str(path), str(error))


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()


def object_value(value, path, fields=None):
    if not isinstance(value, dict):
        fail(path, "expected object")
    if fields is not None and set(value) != set(fields):
        fail(path, f"expected fields {sorted(fields)}, got {sorted(value)}")
    return value


def array(value, path):
    if not isinstance(value, list):
        fail(path, "expected array")
    return value


def integer(value, path, minimum=0):
    if type(value) is not int or value < minimum:
        fail(path, f"expected integer >= {minimum}")
    return value


def string(value, path, nonempty=True):
    if not isinstance(value, str) or (nonempty and not value):
        fail(path, "expected string")
    try:
        value.encode("utf-8")
    except UnicodeError:
        fail(path, "invalid UTF-8 string")
    return value


def boolean(value, path):
    if type(value) is not bool:
        fail(path, "expected bool")
    return value


def labels(values, python):
    return PYTHON_LABELS[tuple(values)] if python else values


def label(value, values, path):
    if value not in values:
        fail(path, f"unsupported label {value!r}")


def validate_timing(row, count=None, path="timing", allow_zero=False):
    object_value(row, path, ("count", *QUANTILES))
    values = [
        integer(row[key], f"{path}.{key}", 0 if allow_zero else 1) for key in QUANTILES
    ]
    actual = integer(row["count"], f"{path}.count", 1)
    if values != sorted(values) or (count is not None and count != actual):
        fail(path, "invalid or undersampled timing")


def inventory(root=ROOT):
    value = load(root / INVENTORY_PATH)
    object_value(
        value,
        "inventory",
        (
            "schema_version",
            "native_long_method",
            "native_method",
            "python_method",
            "native",
            "python",
        ),
    )
    if (
        type(value.get("schema_version")) is not int
        or value["schema_version"] != VERSION
    ):
        fail("inventory", "unsupported format; regenerate with current instrumentation")
    for key, expected in (
        ("native_long_method", "instant-full-call-disposal-100-no-warmup-v1"),
        ("native_method", "instant-frozen-cohorts-v1"),
        ("python_method", "perf-counter-installed-frozen-v1"),
    ):
        if value[key] != expected:
            fail(f"inventory.{key}", "unsupported frozen method")
    method = object_value(value.get("python"), "inventory.python", PYTHON_METHOD)
    for key in (
        "sequential_samples",
        "warmup_calls",
        "constructor_samples",
        "concurrent_samples",
        "workers",
    ):
        integer(method[key], f"inventory.python.{key}", 1)
    if value.get("python") != PYTHON_METHOD:
        fail("inventory.python", "unsupported frozen installed-call methodology")
    keys = []
    for group in array(value.get("native"), "inventory.native"):
        object_value(group, "inventory.group", ("suite", "ids", "bytes", "policies"))
        suite = string(group["suite"], "inventory.suite")
        for case_id in array(group["ids"], "inventory.ids"):
            string(case_id, "inventory.id")
            for size in array(group["bytes"], "inventory.bytes"):
                integer(size, "inventory.bytes")
                for policy in array(group["policies"], "inventory.policies"):
                    label(policy, POLICIES, "inventory.policy")
                    keys.append((suite, case_id, size, policy))
    if not keys or len(keys) != len(set(keys)):
        fail("inventory", "empty or duplicate workload coverage")
    return value, set(keys)


def report_header(report, kind, catalog):
    object_value(report, kind)
    if (
        type(report.get("schema_version")) is not int
        or report["schema_version"] != VERSION
    ):
        fail(kind, "unsupported report format; regenerate native and Python reports")
    if report.get("report_kind") != kind:
        fail(kind, "wrong report kind")
    if report.get("workload_inventory") != catalog:
        fail(kind, "workload inventory differs; use a common instrumented reference")
    expected = catalog["native_method" if kind == "native" else "python_method"]
    if report.get("method_id") != expected:
        fail(kind, "unsupported measurement method")


def coordinates(record, source_bytes, path, python=False):
    if python:
        start, end = record["start_byte"], record["end_byte"]
    else:
        value = object_value(record["range"], f"{path}.range", ("start", "end"))
        start, end = value["start"], value["end"]
    integer(start, f"{path}.start")
    integer(end, f"{path}.end")
    if not start < end <= len(source_bytes):
        fail(path, "invalid original UTF-8 range")
    for endpoint in (start, end):
        if endpoint < len(source_bytes) and source_bytes[endpoint] & 0xC0 == 0x80:
            fail(path, "output endpoint splits an original UTF-8 scalar")
    return start, end


def diagnostics(records, source_bytes, path, python=False, fallback=False):
    previous = 0
    range_fields = ("start_byte", "end_byte") if python else ("range",)
    fields = (
        ("attempted_class", "reason", "original_category", "strategy")
        if fallback
        else ("category", "explanation")
    )
    for index, record in enumerate(array(records, path)):
        location = f"{path}[{index}]"
        object_value(record, location, (*range_fields, *fields))
        start, previous_end = coordinates(record, source_bytes, location, python)
        if start < previous:
            fail(location, "diagnostic ranges overlap or are unordered")
        previous = previous_end
        if fallback:
            label(record["attempted_class"], labels(CLASSES, python), location)
            label(record["reason"], labels(REASONS, python), location)
            label(record["strategy"], labels(STRATEGIES, python), location)
            if record["original_category"] is not None:
                label(record["original_category"], labels(CATEGORIES, python), location)
        else:
            label(record["category"], labels(CATEGORIES, python), location)
            string(record["explanation"], location)


def validate_outcome(value, text, path, python=False, deadline=False, identity=None):
    object_value(value, path)
    source_bytes = text.encode("utf-8")
    size = len(source_bytes)
    if "result" in value:
        object_value(value, path, ("result",))
        result = object_value(
            value["result"],
            f"{path}.result",
            (
                "normalized_text",
                "locale",
                "normalizer_id",
                "complete",
                "segments",
                "issues",
                "fallbacks",
                "fallback_used",
            ),
        )
        for name in ("normalized_text", "locale", "normalizer_id"):
            string(result[name], f"{path}.{name}", name != "normalized_text")
        if identity is not None and result["normalizer_id"] != identity:
            fail(path, "snapshot normalizer identity mismatch")
        for name in ("complete", "fallback_used"):
            boolean(result[name], f"{path}.{name}")
        segments = array(result["segments"], f"{path}.segments")
        previous = 0
        for index, segment in enumerate(segments):
            location = f"{path}.segments[{index}]"
            ranges = ("start_byte", "end_byte") if python else ("range",)
            object_value(segment, location, (*ranges, "kind", "text", "rule_id"))
            start, end = coordinates(segment, source_bytes, location, python)
            if start != previous:
                fail(location, "segments do not form ordered original-source partition")
            previous = end
            label(segment["kind"], labels(KINDS, python), location)
            string(segment["text"], location, False)
            string(segment["rule_id"], location)
        if previous != size or (size and not segments):
            fail(path, "missing source partition")
        if (
            "".join(segment["text"] for segment in segments)
            != result["normalized_text"]
        ):
            fail(path, "segment text differs from normalized text")
        diagnostics(result["issues"], source_bytes, f"{path}.issues", python)
        diagnostics(
            result["fallbacks"], source_bytes, f"{path}.fallbacks", python, True
        )
        if result["complete"] != (not result["issues"]):
            fail(path, "complete disagrees with issues")
        if result["fallback_used"] != bool(result["fallbacks"]):
            fail(path, "fallback_used disagrees with fallbacks")
        return
    if python:
        object_value(value, path, ("error", "message", "issues", "limit_kind"))
        string(value["message"], f"{path}.message")
        code = value["error"]
        label(
            code,
            (
                "invalid_input",
                "invalid_hint",
                "unresolved",
                "limit_exceeded",
                "cancelled",
            ),
            path,
        )
        if code == "limit_exceeded":
            label(value["limit_kind"], ("input", "hints", "candidates", "result"), path)
        elif value["limit_kind"] is not None:
            fail(path, "unexpected limit kind")
        diagnostics(value["issues"], source_bytes, f"{path}.issues", True)
        if (code == "unresolved") != bool(value["issues"]):
            fail(path, "error issues disagree with code")
        if code == "cancelled" and not deadline:
            fail(path, "unexpected cancellation in deterministic measurement")
    else:
        code = value.get("error")
        if code == "Unresolved":
            object_value(value, path, ("error", "issues"))
            diagnostics(value["issues"], source_bytes, f"{path}.issues")
            if not value["issues"]:
                fail(path, "empty unresolved error")
        elif code == "LimitExceeded":
            object_value(value, path, ("error", "limit"))
            label(value["limit"], ("Input", "Hints", "Candidates", "Result"), path)
        else:
            object_value(value, path, ("error",))
            label(code, ("InvalidInput", "InvalidHint", "Cancelled"), path)
            if code == "Cancelled" and not deadline:
                fail(path, "unexpected cancellation in deterministic measurement")


def validate_input(value, path, policy=None, python=False):
    object_value(value, path, ("text", "options"))
    text = string(value["text"], f"{path}.text", False)
    fields = (
        ("ambiguity_policy", "hints", "cancellation", "deadline_ms")
        if python
        else (
            "ambiguity_policy",
            "hints",
        )
    )
    options = object_value(value["options"], f"{path}.options", fields)
    actual_policy = options["ambiguity_policy"]
    label(
        actual_policy, POLICIES if python else ("Preserve", "Reject", "Fallback"), path
    )
    if policy is not None and actual_policy.lower() != policy:
        fail(path, "policy and options disagree")
    if python and (
        options["cancellation"] is not None or options["deadline_ms"] is not None
    ):
        fail(path, "unexpected controlled options in frozen Python cohorts")
    for hint in array(options["hints"], f"{path}.hints"):
        fields = ("start_byte", "end_byte", "kind") if python else ("range", "kind")
        object_value(hint, f"{path}.hint", fields)
        label(hint["kind"], labels(HINT_KINDS, python), f"{path}.hint.kind")
        bounds = hint if python else object_value(hint["range"], path, ("start", "end"))
        for key in ("start_byte", "end_byte") if python else ("start", "end"):
            integer(bounds[key], f"{path}.hint.{key}")
    return text


@dataclass(frozen=True)
class LongRow:
    key: tuple[str, str, int, str]
    input_sha256: str
    outcome: dict
    timing: dict


@dataclass(frozen=True)
class NativeReport:
    value: dict
    scaling: tuple[LongRow, ...]
    inventory_sha256: str


def native_sections(report):
    return (
        (("expanded",), report["expanded_coverage_measurement"]["scaling"]),
        (("large",), report["large"]),
        (("limits",), report["limit_diagnostics"]),
        (("recognition",), report["reviewed_recognition_scaling"]["scaling"]),
        (
            ("scanner", "source-map", "unicode-fallback", "resources"),
            report["supplementary_scaling"],
        ),
    )


def admit_native(report, root=ROOT):
    catalog, required = inventory(root)
    report_header(report, "native", catalog)
    if report.get("method") != NATIVE_METHOD:
        fail("native.method", "sampler methods differ from the frozen method")
    string(report.get("source_sha"), "native.source_sha")
    string(report.get("normalizer_id"), "native.normalizer_id")
    for name in (
        "fallback_measurement",
        "expanded_coverage_measurement",
        "reviewed_recognition_scaling",
    ):
        object_value(report.get(name), f"native.{name}")
    indexed = {}
    try:
        sections = native_sections(report)
    except (KeyError, TypeError) as error:
        fail("native", f"missing scaling section {error}")
    for suites, section in sections:
        for row in array(section, "native.scaling"):
            path = "native.scaling"
            object_value(
                row,
                path,
                (
                    "suite",
                    "id",
                    "bytes",
                    "policy",
                    "method",
                    "input_fingerprint",
                    "timing",
                    "outcome",
                ),
            )
            key = (
                string(row["suite"], path),
                string(row["id"], path),
                integer(row["bytes"], path),
                row["policy"],
            )
            label(row["policy"], POLICIES, path)
            label(row["suite"], suites, path)
            path = f"native.scaling{key}"
            if key in indexed:
                fail(path, "duplicate workload")
            if row["method"] != catalog["native_long_method"]:
                fail(path, "incompatible long-call method")
            text = validate_input(row["input_fingerprint"], path, row["policy"])
            if len(text.encode("utf-8")) != row["bytes"]:
                fail(path, "input byte identity differs")
            validate_timing(row["timing"], 100, path)
            validate_outcome(
                row["outcome"], text, path, identity=report["normalizer_id"]
            )
            indexed[key] = LongRow(
                key,
                digest(row["input_fingerprint"]),
                row["outcome"],
                row["timing"],
            )
    if indexed.keys() != required:
        fail(
            "native",
            f"coverage differs: missing={sorted(required - indexed.keys())}, extra={sorted(indexed.keys() - required)}",
        )
    cases = fixture_cases(root)
    contract, _ = policy_contract_fixture(root)
    for section, policies, expanded in (
        (report, ("preserve", "reject"), False),
        (report["fallback_measurement"], ("fallback",), False),
        (report["expanded_coverage_measurement"], POLICIES, True),
    ):
        expected = prepared_inputs(
            contract if expanded else cases, policies, expanded, False
        )
        if not expanded:
            expected = {
                (
                    key.split(":", 1)[1]
                    if len(policies) == 2
                    else key.split(":", 1)[1].rsplit(":", 1)[0]
                ): value
                for key, value in expected.items()
            }
        validate_snapshots(
            section,
            "native.snapshots",
            expected=expected,
            identity=report["normalizer_id"],
        )
        counts = python_counts(contract if expanded else cases, policies, expanded)
        actual = object_value(section.get("per_class_policy"), "native.classes")
        if actual.keys() != counts.keys():
            fail("native.classes", "class/policy coverage differs")
        for key, count in counts.items():
            validate_timing(actual[key], count, key)
        keys = (
            {f"{c}:{p}" for c in ("short", "medium") for p in policies}
            if expanded
            else {"short", "medium"}
        )
        cohorts = object_value(section.get("cohorts"), "native.cohorts")
        if cohorts.keys() != keys:
            fail("native.cohorts", "cohort coverage differs")
        for timing in cohorts.values():
            validate_timing(timing, 10000, "native.cohort")
    reported_cases = array(
        report["expanded_coverage_measurement"].get("input_cases"),
        "native.policy_cases",
    )

    def input_metadata(cases):
        return [
            {key: case[key] for key in ("id", "text", "class", "hint") if key in case}
            for case in (object_value(case, "native.policy_cases") for case in cases)
        ]

    if input_metadata(reported_cases) != input_metadata(contract):
        fail("native", "policy fixture coverage differs")
    integer(report.get("constructor_first_process_init_ns"), "native.constructor", 1)
    validate_timing(report.get("constructors"), 100, "native.constructors")
    validate_timing(
        report.get("clock_overhead"), 10000, "native.clock", allow_zero=True
    )
    recognition = report["reviewed_recognition_scaling"]
    text = validate_input(
        recognition.get("deadline_input_fingerprint"), "native.deadline.input"
    )
    if (
        type(recognition.get("deadline_ms")) is not int
        or recognition["deadline_ms"] != 10
    ):
        fail("native.deadline", "unsupported observation deadline")
    observations = array(recognition.get("electronic_deadline_10ms"), "native.deadline")
    if len(observations) != 3:
        fail("native.deadline", "expected three separate observations")
    for observation in observations:
        object_value(observation, "native.deadline", ("elapsed_ns", "outcome"))
        integer(observation["elapsed_ns"], "native.deadline.elapsed_ns", 1)
        validate_outcome(
            observation["outcome"],
            text,
            "native.deadline",
            deadline=True,
            identity=report["normalizer_id"],
        )
    return NativeReport(report, tuple(indexed.values()), digest(catalog))


def validate_snapshots(section, path, python=False, expected=None, identity=None):
    inputs = object_value(section.get("inputs"), f"{path}.inputs")
    outcomes = object_value(section.get("snapshots"), f"{path}.snapshots")
    if not inputs or inputs.keys() != outcomes.keys():
        fail(path, "input/outcome snapshot coverage differs")
    if expected is not None and inputs != expected:
        fail(path, "prepared inputs/order/options differ from frozen fixtures")
    for key, fingerprint in inputs.items():
        text = validate_input(fingerprint, f"{path}.{key}", python=python)
        validate_outcome(
            outcomes[key], text, f"{path}.{key}", python, identity=identity
        )


def prepared_inputs(cases, policies, expanded=False, python=True):
    prepared = {}
    for cohort in ("short", "medium"):
        selected = cases if expanded else [c for c in cases if c["cohort"] == cohort]
        pairs = (
            ((case, policy) for policy in policies for case in selected)
            if expanded
            else ((case, policy) for case in selected for policy in policies)
        )
        for case, policy in pairs:
            prefix = PREFIX if expanded and cohort == "medium" else ""
            shift = len(prefix.encode("utf-8"))
            hints = []
            if "hint" in case:
                hint = case["hint"]
                hints = [
                    {
                        "start_byte": hint["start"] + shift,
                        "end_byte": hint["end"] + shift,
                        "kind": hint["kind"],
                    }
                ]
            options = {
                "ambiguity_policy": policy,
                "hints": hints,
                "cancellation": None,
                "deadline_ms": None,
            }
            if not python:
                options = {
                    "ambiguity_policy": policy.capitalize(),
                    "hints": [
                        {
                            "range": {"start": h["start_byte"], "end": h["end_byte"]},
                            "kind": h["kind"].capitalize(),
                        }
                        for h in hints
                    ],
                }
            prepared[f"{cohort}:{case['id']}:{policy}"] = {
                "text": prefix + case["text"],
                "options": options,
            }
    return prepared


def fixture_cases(root=ROOT):
    return [
        case
        for name in ("corpus.json", "intent-corpus.json")
        for case in load(root / "benches" / name)
    ]


def python_counts(cases, policies, expanded=False):
    result = {}
    for cohort in ("short", "medium"):
        selected = cases if expanded else [c for c in cases if c["cohort"] == cohort]
        for policy in policies if expanded else (None,):
            keys = [
                f"{cohort}:{case['class']}:{p}"
                for case in selected
                for p in ((policy,) if expanded else policies)
            ]
            if not keys:
                fail("python.coverage", "empty cohort")
            result.update(Counter(keys[i % len(keys)] for i in range(10000)))
    return result


def admit_python(report, root=ROOT):
    catalog, _ = inventory(root)
    report_header(report, "installed-python", catalog)
    identity = object_value(report.get("identity"), "python.identity")
    for key in (
        "python",
        "implementation",
        "executable",
        "native_path",
        "import_path",
        "build_version",
        "normalizer_id",
        "source_sha",
        "tooling_sha",
    ):
        string(identity.get(key), f"python.identity.{key}")
    native_hash = string(identity.get("native_sha256"), "python.native_sha256")
    if len(native_hash) != 64 or any(c not in "0123456789abcdef" for c in native_hash):
        fail("python.native_sha256", "invalid SHA-256")
    if "site-packages" not in identity["import_path"]:
        fail("python.identity", "facade must come from the installed wheel")
    cases = fixture_cases(root)
    contract, files = policy_contract_fixture(root)
    expected_hashes = {
        name: hashlib.sha256((root / "benches" / name).read_bytes()).hexdigest()
        for name in ("corpus.json", "intent-corpus.json")
    }
    if report.get("corpus_sha256") != expected_hashes:
        fail("python", "stale corpus hashes")
    expected_contract_hash = hashlib.sha256(
        b"".join(file.read_bytes() for file in files)
    ).hexdigest()
    if report.get("policy_contract_sha256") != expected_contract_hash:
        fail("python", "stale policy-contract hash")
    if report.get("methods") != catalog["python"]:
        fail("python", "incompatible installed-call methods")
    for name, policies, expanded in (
        ("measurement", ("preserve", "reject"), False),
        ("fallback_measurement", ("fallback",), False),
        ("expanded_coverage_measurement", POLICIES, True),
    ):
        section = object_value(report.get(name), f"python.{name}")
        selected = contract if expanded else cases
        expected = prepared_inputs(selected, policies, expanded)
        validate_snapshots(
            section, f"python.{name}", True, expected, identity["normalizer_id"]
        )
        if section.get("order") != list(expected):
            fail(f"python.{name}", "frozen order differs")
        counts = python_counts(selected, policies, expanded)
        per_class = object_value(
            section.get("per_class_policy"), f"python.{name}.classes"
        )
        if per_class.keys() != counts.keys():
            fail(f"python.{name}", "class/policy coverage differs")
        for key, count in counts.items():
            validate_timing(per_class[key], count, key)
        cohorts = (
            {f"{c}:{p}" for c in ("short", "medium") for p in policies}
            if expanded
            else {"short", "medium"}
        )
        if object_value(section.get("cohorts"), name).keys() != cohorts:
            fail(name, "cohort coverage differs")
        for key, timing in section["cohorts"].items():
            validate_timing(timing, 10000, key)
        if not expanded:
            integer(section.get("constructor_first_ns"), name, 1)
            validate_timing(
                section.get("subsequent_same_process_constructor"), 100, name
            )
            threads = object_value(section.get("concurrent"), name, ("short", "medium"))
            for timing in threads.values():
                validate_timing(timing, 1000, name)
            if section.get("normalizer_id") != identity["normalizer_id"]:
                fail(name, "normalizer identity mismatch")
    validate_timing(
        report.get("clock_overhead"), 10000, "python.clock", allow_zero=True
    )
    return report
