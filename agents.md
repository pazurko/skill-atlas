# Agent Guidelines for Skill Atlas

Welcome! This document defines the instructions, workflows, and standards for AI agents and contributors working on the **Skill Atlas** codebase.

---

## 1. Context & Architecture Overview

- **README**: Start by reading [`README.md`](./README.md) to understand the project description, installation, CLI usage, and high-level architectural components.
- **Shared Memory (`memory/`)**: Inspect the shared memory directory in [`memory/`](./memory/README.md) to review past context, architectural decisions, and previous feature records before modifying code.
  - For every new feature, architectural change, or task request, **create a new memory document (`memory/NNN-<feature-name>.md`)** or update existing memory documents to preserve context for future agents.
  - Keep the index in `memory/README.md` updated.
- **Specifications (`spec/`)**: Always read the specification files in the `spec/` folder (such as [`spec/cli.md`](./spec/cli.md) and [`spec/memory.md`](./spec/memory.md)) before planning or modifying code.
  - If a feature changes, or if new behavior is introduced, **update the relevant spec in `spec/`** to reflect the new requirements.

---

## 2. Development Workflow

1. **Understand Requirements**: Review the issue/task description alongside the relevant specifications in `spec/` and previous records in `memory/`.
2. **Read & Update Specs & Memory**:
   - Inspect `spec/*.md`. If implementation choices require spec adjustments or expansions, update the specs accordingly.
   - Inspect `memory/*.md`. Create or update the corresponding memory markdown file under `memory/` for the feature or request, keeping `memory/README.md` updated.
3. **Implement Changes**: Make minimal, clean, and well-structured modifications to the Rust source code under `src/`.
4. **Integration Testing**:
   - Write integration tests in the `tests/` directory for **every case defined in the specs**.
   - Ensure positive cases, negative cases, boundary cases, and edge cases (e.g., rate limits, missing repositories, non-interactive mode) are thoroughly tested.
5. **Run Local Validation**: Run tests locally with `cargo test` and verify that all test suites pass with zero warnings or errors.
6. **Branch, Commit & Pull Request**:
   - Create a dedicated feature branch for the changes.
   - Commit changes cleanly and include co-authorship metadata when required.
   - Push the branch to the remote repository and open a pull request.
7. **Remote CI Validation**:
   - Verify that all remote CI checks and tests pass completely green on the remote runners.

---

## 3. Definition of Done (DoD)

A task or pull request is strictly considered **Done** only when:

1. **Local Tests Pass**: All unit and integration tests in `tests/` pass locally with zero errors (`cargo test`).
2. **Pushed CI is Green**: Continuous Integration (CI) checks are completely green for the commit on the remote repository.
3. **CI Remediation**: If CI fails (red):
   - Read the CI execution and test logs.
   - Diagnose the root cause.
   - Implement the fix.
   - Push again and re-verify until CI is green.
