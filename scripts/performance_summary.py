"""Document-first Markdown presentation of an admitted native comparison."""

SUMMARY_ROWS_PER_SUITE = 5


def outcome_label(summary):
    if summary["kind"] == "result":
        return "result (complete)" if summary["complete"] else "result (incomplete)"
    label = f"error: {summary['error']}"
    return f"{label} ({summary['limit']})" if "limit" in summary else label


def render_summary(comparison):
    paired = comparison["repetitions"]
    basis = comparison["comparison_basis"]
    head_sha = comparison["head_sha"]
    lines = [
        "## Native performance comparison",
        "",
        f"Requested base: `{basis['requested_base_sha']}`; measured base: `{basis['measurement_base_sha']}`; head: `{head_sha}`.",
        "",
        basis["explanation"],
        "",
        "Informational only. Ratios compare equivalent inputs/outcomes; changed work is not a speedup.",
        "",
        "| Repetition | Primary cohort | Base p95 (us) | Head p95 (us) | Ratio |",
        "|---|---|---|---|---|",
    ]
    for index, pair in enumerate(paired, 1):
        for row in pair["primary"]:
            ratio = f"{row['p95_ratio']:.2f}" if row["comparable"] else "non-comparable"
            lines.append(
                f"| {index} | {row['cohort']} | {row['base']['p95_ns'] / 1000:.1f} "
                f"| {row['head']['p95_ns'] / 1000:.1f} | {ratio} |"
            )
    lines.append(
        "\nAll checked long-input comparisons (including work kinds and non-comparability reasons): `comparison.json`; complete input/options/outcomes: raw base/head reports. Comparable error timings measure failed work, not successful throughput.\n"
    )
    lines.extend(
        [
            "| Repetition | Suite | Compared rows | Non-comparable rows |",
            "|---|---|---:|---:|",
        ]
    )
    for index, pair in enumerate(paired, 1):
        suites = sorted({row["suite"] for row in pair["scaling"]})
        for suite in suites:
            selected = [row for row in pair["scaling"] if row["suite"] == suite]
            lines.append(
                f"| {index} | {suite} | {len(selected)} | {sum(not row['comparable'] for row in selected)} |"
            )
    lines.extend(
        [
            "",
            f"Largest {SUMMARY_ROWS_PER_SUITE} comparable p95 ratios per suite/repetition below (informational, not adoption thresholds). All rows and non-comparability reasons remain in `comparison.json`.",
            "",
            "| Repetition | Suite / workload | Bytes | Policy | Base work | Head work | Base p95 (us) | Head p95 (us) | Ratio |",
            "|---|---|---:|---|---|---|---:|---:|---:|",
        ]
    )
    for index, pair in enumerate(paired, 1):
        for suite in sorted({row["suite"] for row in pair["scaling"]}):
            selected = sorted(
                (
                    row
                    for row in pair["scaling"]
                    if row["suite"] == suite and row["comparable"]
                ),
                key=lambda row: row["p95_ratio"],
                reverse=True,
            )[:SUMMARY_ROWS_PER_SUITE]
            for row in selected:
                lines.append(
                    f"| {index} | {suite} / {row['id']} | {row['bytes']} | {row['policy']} "
                    f"| {outcome_label(row['base_outcome'])} | {outcome_label(row['head_outcome'])} "
                    f"| {row['base']['p95_ns'] / 1000:.1f} | {row['head']['p95_ns'] / 1000:.1f} | {row['p95_ratio']:.2f} |"
                )
    summary = "\n".join(lines)
    return summary
