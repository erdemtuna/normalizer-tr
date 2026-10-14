"""Validated semantic comparisons, independent of sampling and presentation."""

from .common import ROOT
from .compare_contracts import semantic_outcome
from .performance_report import admit_native, validate_timing

SAMPLES = 10000


def snapshots_equal(base, head):
    return {key: semantic_outcome(value) for key, value in base.items()} == {
        key: semantic_outcome(value) for key, value in head.items()
    }


def timing_rows(base, head, comparable):
    if base.keys() != head.keys():
        raise RuntimeError("measurement cohorts differ")
    rows = []
    for key in base:
        validate_timing(base[key])
        validate_timing(head[key])
        if base[key]["count"] != head[key]["count"]:
            raise RuntimeError(f"sample counts differ: {key}")
        rows.append(
            {
                "cohort": key,
                "comparable": comparable,
                "base": base[key],
                "head": head[key],
                "p95_ratio": head[key]["p95_ns"] / base[key]["p95_ns"]
                if comparable
                else None,
            }
        )
    return rows


def outcome_summary(outcome):
    if "result" in outcome:
        return {"kind": "result", "complete": outcome["result"]["complete"]}
    summary = {"kind": "error", "error": outcome["error"]}
    if "limit" in outcome:
        summary["limit"] = outcome["limit"]
    return summary


def compare(
    base, head, same_inputs, contract_changes=(), *, base_root=ROOT, head_root=ROOT
):
    before_report = admit_native(base, base_root)
    after_report = admit_native(head, head_root)
    if base["method"] != head["method"]:
        raise RuntimeError("sampler methods differ")
    for report in (base, head):
        if set(report["cohorts"]) != {"short", "medium"}:
            raise RuntimeError("missing short or medium cohort")
        for section in (
            report["cohorts"],
            report["fallback_measurement"]["cohorts"],
            report["expanded_coverage_measurement"]["cohorts"],
        ):
            for row in section.values():
                validate_timing(row, SAMPLES)
        if set(report["fallback_measurement"]["cohorts"]) != {"short", "medium"}:
            raise RuntimeError("missing fallback cohorts")
        expected_policies = {
            f"{cohort}:{policy}"
            for cohort in ("short", "medium")
            for policy in ("preserve", "reject", "fallback")
        }
        if set(report["expanded_coverage_measurement"]["cohorts"]) != expected_policies:
            raise RuntimeError("missing policy contract cohorts")
    same_primary = (
        same_inputs
        and base["distribution"] == head["distribution"]
        and snapshots_equal(base["snapshots"], head["snapshots"])
    )
    same_fallback = same_inputs and snapshots_equal(
        base["fallback_measurement"]["snapshots"],
        head["fallback_measurement"]["snapshots"],
    )
    same_policy = (
        (
            base["expanded_coverage_measurement"]["input_cases"]
            == head["expanded_coverage_measurement"]["input_cases"]
        )
        and (
            base["expanded_coverage_measurement"]["inputs"]
            == head["expanded_coverage_measurement"]["inputs"]
        )
        and snapshots_equal(
            base["expanded_coverage_measurement"]["snapshots"],
            head["expanded_coverage_measurement"]["snapshots"],
        )
        and not any(
            row["id"].startswith(("base:", "head:")) for row in contract_changes
        )
    )
    result = {
        "primary": timing_rows(base["cohorts"], head["cohorts"], same_primary),
        "fallback": timing_rows(
            base["fallback_measurement"]["cohorts"],
            head["fallback_measurement"]["cohorts"],
            same_fallback,
        ),
        "policy_contract": timing_rows(
            base["expanded_coverage_measurement"]["cohorts"],
            head["expanded_coverage_measurement"]["cohorts"],
            same_policy,
        ),
        "per_class_policy": {
            "shared": timing_rows(
                {
                    key: value
                    for key, value in base["per_class_policy"].items()
                    if key in head["per_class_policy"]
                },
                {
                    key: head["per_class_policy"][key]
                    for key in base["per_class_policy"]
                    if key in head["per_class_policy"]
                },
                same_primary,
            ),
            "base_only": sorted(
                base["per_class_policy"].keys() - head["per_class_policy"].keys()
            ),
            "head_only": sorted(
                head["per_class_policy"].keys() - base["per_class_policy"].keys()
            ),
        },
    }
    before = {row.key: row for row in before_report.scaling}
    after = {row.key: row for row in after_report.scaling}
    if tuple(before) != tuple(after):
        raise RuntimeError("long workload order differs")
    result["scaling"] = []
    for key, row in before.items():
        other = after[key]
        reasons = []
        if row.input_sha256 != other.input_sha256:
            reasons.append("input/options changed")
        if semantic_outcome(row.outcome) != semantic_outcome(other.outcome):
            reasons.append("semantic outcome changed")
        result["scaling"].append(
            {
                "suite": key[0],
                "id": key[1],
                "bytes": key[2],
                "policy": key[3],
                "base_input_sha256": row.input_sha256,
                "head_input_sha256": other.input_sha256,
                "base_outcome": outcome_summary(row.outcome),
                "head_outcome": outcome_summary(other.outcome),
                "reason": "; ".join(reasons) or None,
                **timing_rows(
                    {str(key): row.timing}, {str(key): other.timing}, not reasons
                )[0],
            }
        )
    return result
