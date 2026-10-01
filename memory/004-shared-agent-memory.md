# Shared Agent Memory System

- **Status**: Implemented
- **Date**: 2026-10-01
- **Author/Agent**: Core Development Team
- **PR / Issue Reference**: Shared Memory System Initiative

---

## 1. Context & Motivation
As AI agents and human contributors collaborate across different sessions and tasks, valuable architectural insights, constraints, and operational knowledge can be lost between sessions. The `/memory` directory provides a persistent, human- and machine-readable memory system where each feature, architectural update, or request is recorded in structured Markdown documents.

## 2. Requirements & Specification
- Dedicated `/memory` directory at the project root.
- Clear file naming convention: `NNN-<feature-name>.md` with continuous numerical ordering.
- `memory/README.md` acting as the central index and guidelines for documenting future features.
- Mandatory agent workflow:
  - Read existing memory files before planning or implementing changes.
  - Create or update the corresponding `.md` file in `/memory` for every new feature or significant request.
  - Keep the index in `memory/README.md` up to date.
- Explicit guidelines integrated into `agents.md` and `README.md`.

## 3. Architecture & Implementation Details
- `memory/README.md`: Central overview, standard template, and document index.
- `memory/001-core-scanner-and-caching.md`: Foundational architecture of CLI, SQLite caching, and GitHub scanner.
- `memory/002-localhost-web-interface.md`: Localhost Axum web server and single-page application.
- `memory/003-filter-and-similarity-skills.md`: Skill filtering and heuristic similarity metrics.
- `memory/004-shared-agent-memory.md`: Specification and operation of the shared memory system.
- `spec/memory.md`: Formal specification of the memory subsystem.
- `agents.md`: Updated agent workflow requiring memory consultation and updates.

## 4. Testing & Verification
- Validated structure, links, and markdown syntax across all memory documents.
- Ensured all existing unit and integration test suites in `tests/` continue to pass with zero errors.

## 5. Notes & Future Recommendations
- Subsequent agents should always check `memory/` when starting new work to understand previous architectural decisions.
- Keep memory files concise, structured, and focused on design rationale and implementation facts.
