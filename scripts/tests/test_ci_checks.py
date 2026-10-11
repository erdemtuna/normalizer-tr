import unittest
import tempfile
from pathlib import Path

from scripts.ci_checks import check_tool_pins, dependency_changes, require_success


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
            "bindings/python/requirements-dev.txt",
            "bindings/python/constraints.lock",
            ".github/workflows/release.yml",
            ".github/dependabot.yml",
            "rust-toolchain.toml",
        ):
            with self.subTest(path=path):
                self.assertTrue(dependency_changes([path]))
        self.assertFalse(dependency_changes(["README.md", "src/pipeline.rs"]))

    def test_secondary_tool_pins_cannot_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binding = root / "bindings/python"
            binding.mkdir(parents=True)
            workflows = root / ".github/workflows"
            workflows.mkdir(parents=True)
            (binding / "requirements-dev.txt").write_text("maturin==1.15.0\n")
            lock = binding / "constraints.lock"
            lock.write_text("maturin==1.15.0\n")
            workflow = workflows / "test.yml"
            workflow.write_text(
                'maturin-version: "1.15.0"\nrun: pip install maturin==1.15.0\n'
            )
            check_tool_pins(root)
            lock.write_text("maturin==1.14.0\n")
            with self.assertRaisesRegex(RuntimeError, "constraint"):
                check_tool_pins(root)
            lock.write_text("maturin==1.15.0\n")
            workflow.write_text('maturin-version: "1.14.0"\n')
            with self.assertRaisesRegex(RuntimeError, "maturin"):
                check_tool_pins(root)


if __name__ == "__main__":
    unittest.main()
