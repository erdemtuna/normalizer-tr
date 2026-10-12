# Copilot instructions

- **Keep language rules in Rust.** Reuse typed lexicon/morphology helpers; no Python or scanner special cases.
- **Test the installed binding.** Use one Python environment and rebuild/reinstall after native changes, even without a version bump.
- **Reproduce before claiming.** Verify the source revision and loaded native module; release notes are not proof of a fix, and text tests are not audio validation.
- **Protect source ownership.** Invalid compounds stay atomic. New readers must not steal protected identifiers, list members or adjacent address fields.
- **Check all three policies.** Assert text, completeness, diagnostics and original UTF-8 ranges; fallback must not hide cancellation or limit errors.
- **Measure scaling, not just warm p95.** Include malformed/long inputs; compare equivalent base/head workloads serially and preserve frozen benchmark inputs.

Use `CONTRIBUTING.md` for commands and releases, `PERFORMANCE.md` for measurement,
and `docs\normalization.md` / `docs\fallback.md` for behavior. Keep detailed
guidance there, not in this file or the README.
