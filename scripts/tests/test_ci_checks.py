import unittest
import tempfile
import json
import subprocess
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from scripts.ci_checks import (
    check_installed_tools,
    check_tool_pins,
    dependency_changes,
    main,
    performance_changes,
    performance_scope_base,
    read_requirements,
    require_success,
    tool_version,
)
from scripts.common import ROOT


class CiTests(unittest.TestCase):
    def test_performance_reference_output_is_explicit(self):
        from scripts.compare_performance import PRE_V2_BASE_SHA, V2_REFERENCE_SHA

        for pr_base, requested, expected in (
            (PRE_V2_BASE_SHA, "", V2_REFERENCE_SHA),
            ("other-base", "ignored", "other-base"),
            ("", PRE_V2_BASE_SHA, PRE_V2_BASE_SHA),
            ("", "v0.7.0", "v0.7.0"),
        ):
            with (
                self.subTest(pr_base=pr_base, requested=requested),
                tempfile.TemporaryDirectory() as directory,
            ):
                output = Path(directory) / "outputs"
                with (
                    patch("sys.argv", ["ci-checks", "performance-reference"]),
                    patch.dict(
                        "os.environ",
                        {
                            "PR_BASE_SHA": pr_base,
                            "REQUESTED_BASE_REF": requested,
                            "GITHUB_OUTPUT": str(output),
                        },
                    ),
                ):
                    main()
                self.assertEqual(output.read_text(), f"reference={expected}\n")

    def test_missing_performance_reference_cannot_emit_success(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "outputs"
            with (
                patch("sys.argv", ["ci-checks", "performance-reference"]),
                patch.dict(
                    "os.environ",
                    {
                        "PR_BASE_SHA": "",
                        "REQUESTED_BASE_REF": "",
                        "GITHUB_OUTPUT": str(output),
                    },
                ),
                self.assertRaisesRegex(RuntimeError, "exact base ref"),
            ):
                main()
            self.assertFalse(output.exists())

    def test_gate_requires_every_job_to_succeed(self):
        require_success(
            {"core": {"result": "success"}, "binding": {"result": "success"}}
        )
        for status in ("failure", "cancelled", "skipped", "pending"):
            with self.subTest(status=status), self.assertRaises(RuntimeError):
                require_success(
                    {"core": {"result": "success"}, "binding": {"result": status}}
                )
        with self.assertRaises(RuntimeError):
            require_success({})

    def test_dependency_selection_covers_manifests_constraints_and_workflows(self):
        for path in (
            "Cargo.toml",
            "Cargo.lock",
            "bindings/python/Cargo.toml",
            "bindings/python/pyproject.toml",
            "bindings/python/requirements-dev.in",
            "bindings/python/requirements-dev.txt",
            "bindings/python/constraints.lock",
            ".github/workflows/release.yml",
            ".github/dependabot.yml",
            "rust-toolchain.toml",
        ):
            with self.subTest(path=path):
                self.assertTrue(dependency_changes([path]))
        self.assertFalse(dependency_changes(["README.md", "src/pipeline.rs"]))


class PerformanceScopeTests(unittest.TestCase):
    before = "a" * 40
    pr_base = "b" * 40

    def test_measurement_code_data_and_tooling_are_selected(self):
        for path in (
            "src/lib.rs",
            "src/data/resources.json",
            "benches/latency.rs",
            "benches/latency/source_mapping.rs",
            "benches/latency/workload_inventory.json",
            "benches/corpus.json",
            "benches/intent-corpus.json",
            "tests/fixtures/policy-contract/money.json",
            "tests/support/policy_contract.rs",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "scripts/compare_performance.py",
            "scripts/compare_contracts.py",
            "scripts/common.py",
            "scripts/ci_checks.py",
            "scripts/performance_comparison.py",
            "scripts/performance_report.py",
            "scripts/performance_summary.py",
            "scripts/verification.py",
            "scripts/verify.ps1",
            "scripts/rust/contract_probe.rs",
            ".github/workflows/performance.yml",
        ):
            with self.subTest(path=path):
                self.assertTrue(performance_changes([path]))

    def test_docs_media_and_unrelated_checks_are_not_selected(self):
        paths = [
            "README.md",
            "CHANGELOG.md",
            "CONTRIBUTING.md",
            "PERFORMANCE.md",
            "docs/fallback.md",
            "assets/social-preview.png",
            "bindings/python/README.md",
            "bindings/python/python/normalizer_tr/__init__.py",
            "benches/python_latency.py",
            "benches/latency/README.md",
            "src/data/provenance.md",
            "tests/fallback.rs",
            "scripts/tests/test_ci_checks.py",
            ".github/workflows/ci.yml",
        ]
        self.assertFalse(performance_changes(paths))
        self.assertFalse(performance_changes([]))
        self.assertTrue(performance_changes(paths + ["src/pipeline.rs"]))

    def test_synchronize_uses_previous_head_not_the_accumulated_pr_base(self):
        self.assertEqual(
            performance_scope_base(
                "pull_request", "synchronize", self.before, self.pr_base
            ),
            self.before,
        )
        for action in ("opened", "reopened"):
            self.assertEqual(
                performance_scope_base(
                    "pull_request", action, self.before, self.pr_base
                ),
                self.pr_base,
            )
        self.assertIsNone(performance_scope_base("workflow_dispatch", None, None, None))

    def test_missing_malformed_and_unsupported_events_fail(self):
        for value in (None, "", "HEAD", "a" * 39, True):
            with (
                self.subTest(value=value),
                self.assertRaisesRegex(RuntimeError, "exact event base"),
            ):
                performance_scope_base(
                    "pull_request", "synchronize", value, self.pr_base
                )
        for event_name, action in (("push", None), ("pull_request", "closed")):
            with self.assertRaisesRegex(RuntimeError, "unsupported"):
                performance_scope_base(event_name, action, self.before, self.pr_base)

    def run_scope(
        self, directory, paths, *, actual=None, failure=None, diff_failure=None
    ):
        root = Path(directory)
        event_path = root / "event.json"
        event_path.write_text(
            json.dumps(
                {
                    "action": "synchronize",
                    "before": self.before,
                    "pull_request": {"base": {"sha": self.pr_base}},
                }
            )
        )
        output, summary = root / "outputs", root / "summary"
        checkout = (root / "base").resolve()
        commands = patch(
            "scripts.ci_checks.subprocess.check_output",
            side_effect=[
                (actual or self.before) + "\n",
                diff_failure or "\0".join(paths) + "\0",
            ],
        )
        with (
            patch(
                "sys.argv",
                ["ci-checks", "performance-scope", "--base-checkout", str(checkout)],
            ),
            patch.dict(
                "os.environ",
                {
                    "GITHUB_EVENT_NAME": "pull_request",
                    "GITHUB_EVENT_PATH": str(event_path),
                    "GITHUB_OUTPUT": str(output),
                    "GITHUB_STEP_SUMMARY": str(summary),
                },
            ),
            commands as reads,
            patch("scripts.ci_checks.subprocess.run", side_effect=failure) as fetch,
        ):
            main()
        fetch.assert_called_once_with(
            ["git", "fetch", "--no-tags", "--update-shallow", str(checkout), "HEAD"],
            cwd=ROOT,
            check=True,
        )
        self.assertEqual(
            reads.call_args_list[-1].args[0],
            [
                "git",
                "diff",
                "--no-renames",
                "--name-only",
                "-z",
                self.before,
                "HEAD",
                "--",
            ],
        )
        return output.read_text(), summary.read_text()

    def test_actual_cli_skips_docs_and_preserves_deleted_or_renamed_source_paths(self):
        for paths, expected in (
            (["README.md", "CHANGELOG.md"], "false"),
            (["src/pipeline.rs", "README.md"], "true"),
            (["src/renamed\nsource.rs"], "true"),
            (["scripts/performance_comparison.py"], "true"),
            (["scripts/performance_summary.py"], "true"),
        ):
            with self.subTest(paths=paths), tempfile.TemporaryDirectory() as directory:
                output, summary = self.run_scope(directory, paths)
                self.assertEqual(output, f"run={expected}\n")
                self.assertIn("latest push", summary)
                self.assertIn(self.before, summary)

    def test_checkout_mismatch_or_git_failure_cannot_emit_a_skip(self):
        for actual, failure, diff_failure, error in (
            (self.pr_base, None, None, RuntimeError),
            (
                None,
                subprocess.CalledProcessError(1, ["git", "fetch"]),
                None,
                subprocess.CalledProcessError,
            ),
            (
                None,
                None,
                subprocess.CalledProcessError(1, ["git", "diff"]),
                subprocess.CalledProcessError,
            ),
        ):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                with self.assertRaises(error):
                    self.run_scope(
                        directory,
                        ["README.md"],
                        actual=actual,
                        failure=failure,
                        diff_failure=diff_failure,
                    )
                self.assertFalse((root / "outputs").exists())
                self.assertFalse((root / "summary").exists())

    def test_manual_dispatch_requests_measurement_without_diffing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with (
                patch("sys.argv", ["ci-checks", "performance-scope"]),
                patch.dict(
                    "os.environ",
                    {
                        "GITHUB_EVENT_NAME": "workflow_dispatch",
                        "GITHUB_OUTPUT": str(root / "outputs"),
                        "GITHUB_STEP_SUMMARY": str(root / "summary"),
                    },
                ),
                patch("scripts.ci_checks.subprocess.run") as fetch,
                patch("scripts.ci_checks.subprocess.check_output") as reads,
            ):
                main()
            fetch.assert_not_called()
            reads.assert_not_called()
            self.assertEqual((root / "outputs").read_text(), "run=true\n")
            self.assertIn("Manual dispatch", (root / "summary").read_text())

    def test_only_the_conditionally_run_heavy_job_owns_concurrency(self):
        workflow = (ROOT / ".github/workflows/performance.yml").read_text()
        self.assertNotIn("\nconcurrency:", workflow)
        compare = workflow.split("\n  compare:", 1)[1]
        self.assertIn("needs: scope", compare)
        self.assertIn("if: needs.scope.outputs.run == 'true'", compare)
        self.assertIn("    concurrency:", compare)
        self.assertIn("      cancel-in-progress: true", compare)

    def test_real_shallow_force_push_snapshots_use_endpoint_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, head, base = root / "source", root / "head", root / "base"
            source.mkdir()

            def git(*args, cwd=source):
                return subprocess.check_output(
                    ["git", *args], cwd=cwd, text=True, stderr=subprocess.PIPE
                ).strip()

            git("init", "--quiet")
            git("config", "user.name", "Scope fixture")
            git("config", "user.email", "scope@example.invalid")
            (source / "src").mkdir()
            code = source / "src" / "pipeline.rs"
            code.write_text("old\n")
            (source / "README.md").write_text("old\n")
            git("add", ".")
            git("commit", "--quiet", "-m", "initial")
            initial = git("rev-parse", "HEAD")
            code.write_text("new\n")
            git("commit", "--quiet", "-am", "code")
            previous = git("rev-parse", "HEAD")
            git("branch", "previous")
            git("switch", "--detach", initial)
            code.write_text("new\n")
            (source / "README.md").write_text("new docs\n")
            git("commit", "--quiet", "-am", "rewritten docs")
            git("branch", "current")
            for branch, target in (("current", head), ("previous", base)):
                git(
                    "clone",
                    "--quiet",
                    "--depth",
                    "1",
                    "--branch",
                    branch,
                    source.as_uri(),
                    str(target),
                    cwd=root,
                )
            event_path = root / "event.json"
            event_path.write_text(
                json.dumps(
                    {
                        "action": "synchronize",
                        "before": previous,
                        "pull_request": {"base": {"sha": initial}},
                    }
                )
            )
            output = root / "outputs"
            with (
                patch("scripts.ci_checks.ROOT", head),
                patch(
                    "sys.argv",
                    [
                        "ci-checks",
                        "performance-scope",
                        "--base-checkout",
                        str(base),
                    ],
                ),
                patch.dict(
                    "os.environ",
                    {
                        "GITHUB_EVENT_NAME": "pull_request",
                        "GITHUB_EVENT_PATH": str(event_path),
                        "GITHUB_OUTPUT": str(output),
                        "GITHUB_STEP_SUMMARY": str(root / "summary"),
                    },
                ),
            ):
                main()
            self.assertEqual(output.read_text(), "run=false\n")


class ToolPinTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        binding = self.root / "bindings/python"
        binding.mkdir(parents=True)
        workflows = self.root / ".github/workflows"
        workflows.mkdir(parents=True)
        self.source = binding / "requirements-dev.in"
        self.lock = binding / "requirements-dev.txt"
        self.source.write_text("maturin==1.15.0\npytest==9.1.1\n")
        self.lock.write_text("# Generated lock\nmaturin==1.15.0\npytest==9.1.1\n")
        self.build = binding / "pyproject.toml"
        self.build.write_text('[build-system]\nrequires = ["maturin==1.15.0"]\n')
        self.workflow = workflows / "test.yml"
        self.workflow.write_text(
            "run: python -m pip install --constraint "
            "bindings/python/requirements-dev.txt maturin pytest\n"
        )

    def test_source_and_lock_must_agree(self):
        check_tool_pins(self.root)
        for text in ("maturin==1.14.0\npytest==9.1.1\n", "maturin==1.15.0\n"):
            with self.subTest(text=text):
                self.lock.write_text(text)
                with self.assertRaisesRegex(RuntimeError, "lock differs"):
                    check_tool_pins(self.root)

    def test_build_pin_must_agree_and_be_unique(self):
        for requires in (
            '["maturin==1.14.0"]',
            '["maturin>=1.15.0"]',
            '["maturin==1.15.0", "maturin==1.14.0"]',
        ):
            with self.subTest(requires=requires):
                self.build.write_text(f"[build-system]\nrequires = {requires}\n")
                with self.assertRaisesRegex(RuntimeError, "build-system"):
                    check_tool_pins(self.root)

    def test_duplicate_installer_and_action_versions_are_rejected(self):
        for text in (
            "run: pip install pytest==9.1.1\n",
            "run: pip install pytest==9.0.3\n",
            'maturin-version: "1.15.0"\n',
        ):
            with self.subTest(text=text):
                self.workflow.write_text(text)
                with self.assertRaisesRegex(RuntimeError, "duplicates"):
                    check_tool_pins(self.root)

    def test_retired_constraint_references_are_rejected(self):
        self.workflow.write_text("run: pip install -c constraints.lock pytest\n")
        with self.assertRaisesRegex(RuntimeError, "retired"):
            check_tool_pins(self.root)

    def test_retired_constraint_file_is_rejected(self):
        (self.lock.parent / "constraints.lock").write_text("pytest==9.0.3\n")
        with self.assertRaisesRegex(RuntimeError, "retired"):
            check_tool_pins(self.root)

    def test_local_consumer_cannot_keep_a_copied_pin(self):
        scripts = self.root / "scripts"
        scripts.mkdir()
        (scripts / "verify.ps1").write_text("pip install pytest==9.1.1\n")
        with self.assertRaisesRegex(RuntimeError, "verify.ps1"):
            check_tool_pins(self.root)

    def test_tool_upgrade_needs_no_workflow_version_edit(self):
        workflow = self.workflow.read_text()
        self.source.write_text("maturin==1.15.0\npytest==9.2.0\n")
        self.lock.write_text("maturin==1.15.0\npytest==9.2.0\n")
        check_tool_pins(self.root)
        self.assertEqual(tool_version("pytest", self.root), "9.2.0")
        self.assertEqual(self.workflow.read_text(), workflow)

    def test_tool_version_requires_a_matching_direct_pin(self):
        with self.assertRaisesRegex(RuntimeError, "no direct"):
            tool_version("missing", self.root)
        self.lock.write_text("maturin==1.15.0\npytest==9.0.3\n")
        with self.assertRaisesRegex(RuntimeError, "lock differs"):
            tool_version("pytest", self.root)

    def test_pins_reject_duplicates_malformed_and_unpinned_tools(self):
        for text in (
            "",
            "maturin==1.15.0\nmaturin==1.15.0\n",
            "mypy-extensions==1.1.0\nmypy_extensions==1.1.0\n",
            "pytest>=9.1.1\n",
            "pytest\n",
            "pytest==9.1.1; sys_platform == 'win32'\n",
        ):
            with self.subTest(text=text):
                self.source.write_text(text)
                with self.assertRaises(RuntimeError):
                    read_requirements(self.source, source=True)

    def test_portability_inputs_still_require_resolved_pins(self):
        self.source.write_text("maturin==1.15.0\ncolorama\n")
        self.lock.write_text("maturin==1.15.0\ncolorama==0.4.6\n")
        check_tool_pins(self.root)
        self.lock.write_text("maturin==1.15.0\ncolorama\n")
        with self.assertRaisesRegex(RuntimeError, "missing exact"):
            check_tool_pins(self.root)

    def installed(self, distributions, tools=("maturin",)):
        with patch(
            "scripts.ci_checks.metadata.distributions", return_value=distributions
        ):
            check_installed_tools(list(tools), self.root)

    @staticmethod
    def distribution(name, version, requires=()):
        return SimpleNamespace(
            metadata={"Name": name}, version=version, requires=list(requires)
        )

    def test_installed_dependency_closure_is_pinned(self):
        self.lock.write_text("maturin==1.15.0\npytest==9.1.1\nsome-package==2.0\n")
        distributions = [
            self.distribution("maturin", "1.15.0", ["some_package>=2"]),
            self.distribution("Some.Package", "2.0"),
        ]
        self.installed(distributions)
        for text in (
            "maturin==1.15.0\npytest==9.1.1\n",
            "maturin==1.15.0\npytest==9.1.1\nsome-package==3.0\n",
        ):
            with self.subTest(text=text):
                self.lock.write_text(text)
                with self.assertRaisesRegex(RuntimeError, "pinned|differs"):
                    self.installed(distributions)

    def test_missing_and_unsatisfied_dependencies_fail(self):
        for distributions in (
            [],
            [self.distribution("maturin", "1.15.0", ["some-package>=2"])],
            [
                self.distribution("maturin", "1.15.0", ["some-package>=2"]),
                self.distribution("some-package", "1.0"),
            ],
        ):
            with self.subTest(distributions=distributions):
                with self.assertRaisesRegex(RuntimeError, "pinned|unsatisfied"):
                    self.installed(distributions)

    def test_markers_and_transitive_extras_are_respected(self):
        self.lock.write_text(
            "maturin==1.15.0\npytest==9.1.1\ncachecontrol==1.0\nfilelock==2.0\n"
        )
        distributions = [
            self.distribution(
                "maturin",
                "1.15.0",
                ["cachecontrol[filecache]>=1", "absent; python_version < '3.0'"],
            ),
            self.distribution(
                "cachecontrol",
                "1.0",
                ["filelock>=2; extra == 'filecache'", "absent; extra == 'test'"],
            ),
            self.distribution("filelock", "2.0"),
        ]
        self.installed(distributions)
        with self.assertRaisesRegex(RuntimeError, "unsatisfied"):
            self.installed(distributions[:-1])

    def test_cycles_and_multiple_extra_paths_terminate(self):
        self.lock.write_text(
            "maturin==1.15.0\npytest==9.1.1\nother==1.0\nextra-package==1.0\n"
        )
        self.installed(
            [
                self.distribution("maturin", "1.15.0", ["other", "other[optional]"]),
                self.distribution(
                    "other", "1.0", ["maturin", "extra-package; extra == 'optional'"]
                ),
                self.distribution("extra-package", "1.0"),
            ]
        )

    def test_unknown_or_empty_tool_selection_fails(self):
        for tools in ([], ["unknown"]):
            with self.subTest(tools=tools):
                with self.assertRaisesRegex(RuntimeError, "selection"):
                    check_installed_tools(tools, self.root)

    def test_default_selection_checks_all_declared_tools(self):
        distributions = [
            self.distribution("maturin", "1.15.0"),
            self.distribution("pytest", "9.0.3"),
        ]
        with (
            patch(
                "scripts.ci_checks.metadata.distributions", return_value=distributions
            ),
            self.assertRaisesRegex(RuntimeError, "differs"),
        ):
            check_installed_tools(root=self.root)


if __name__ == "__main__":
    unittest.main()
