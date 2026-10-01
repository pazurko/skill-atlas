# Exclusion and Cleanup of Acme Test Skills

- **Status**: Implemented
- **Date**: 2026-10-01
- **Author/Agent**: Junie
- **PR / Issue Reference**: Cleanup Acme Test Skills

---

## 1. Context & Motivation
- Test fixture data and mock repositories using the `acme` organization name (such as `acme/big`, `acme/many`, `acme/sample`) are intended solely for testing and internal fixture verification.
- When inspecting the local SQLite database catalogue (`~/.skill-atlas/skills.db`) or viewing saved skills in the web UI via "Show all saved", placeholder skills (`skill-00` through `skill-39`, `deploy`) from test runs were visible to users.
- Furthermore, the filter placeholder example in the web interface UI referenced `repo:acme`.

---

## 2. Requirements & Changes
1. **Purge Test Skills from Persistent Database**:
   - Removed test repository entries and associated skill records matching `acme` from `~/.skill-atlas/skills.db`.
2. **Update Web UI Examples**:
   - Updated the filter input placeholder in `src/index.html` to reference production repositories (e.g., `repo:kotlin` or `repo:JetBrains`) rather than test placeholder `repo:acme`.

---

## 3. Architecture & Implementation Details
- `~/.skill-atlas/skills.db`: Executed database cleanup of `acme` test records while preserving authentic scanned repositories (`jetbrains/kotlin`, `jetbrains/mps`).
- `src/index.html`: Refined query filter placeholder text.
- `memory/007-cleanup-test-skills-and-data.md`: Documented the context and cleanup actions.

---

## 4. Testing & Verification
- Validated SQLite database state to confirm only real repositories exist.
- Ran full test suite via `cargo test` (all 74 unit and integration tests passing).
- Checked code quality and formatting with `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings`.
