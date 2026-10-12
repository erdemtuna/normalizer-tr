import unittest
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from scripts.ci_checks import (
    check_installed_tools,
    check_tool_pins,
    dependency_changes,
    read_requirements,
    require_success,
    tool_version,
)


class CiTests(unittest.TestCase):
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
