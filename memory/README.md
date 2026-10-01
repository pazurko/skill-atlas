# Skill Atlas Shared Memory

Welcome to the **Shared Memory** repository for the Skill Atlas project.

This directory serves as the persistent cross-agent and cross-session knowledge base. Whenever an AI agent or human contributor plans, implements, refactors, or responds to a new feature request, issue, or architectural shift, they record information here for future sessions.

---

## 📌 Purpose & Objectives

1. **Context Retention**: Preserves high-signal architectural decisions, rationale, data schemas, edge cases, and testing strategies across sessions.
2. **Onboarding & Fast Discovery**: Enables incoming agents and contributors to instantly understand why components exist and how they work without reverse-engineering the codebase.
3. **Continuous Record of Evolution**: Tracks chronological progress, past bugs, remediations, and future follow-ups.

---

## 📂 Memory Structure & Naming Convention

Memory documents are stored in `/memory` as Markdown files (`.md`).

### Naming Format:
- Use sequential numeric prefixes with kebab-case descriptors:
  - `001-core-scanner-and-caching.md`
  - `002-localhost-web-interface.md`
  - `003-filter-and-similarity-skills.md`
  - `004-shared-agent-memory.md`
  - `NNN-<feature-or-request-name>.md`

---

## 📝 Document Template for New Features / Requests

Every new memory document should follow this standardized outline:

```markdown
# <Feature or Request Title>

- **Status**: [Proposed | Implemented | Deprecated | Superseded]
- **Date**: YYYY-MM-DD
- **Author/Agent**: Agent / Contributor name
- **PR / Issue Reference**: PR #X / Issue #Y

---

## 1. Context & Motivation
- Why was this feature or request needed?
- What problem does it solve?

## 2. Requirements & Specification
- Summary of core functional and non-functional requirements.
- CLI flags, API endpoints, REPL commands, or UI behaviors introduced.

## 3. Architecture & Implementation Details
- Key source files created or modified (`src/...`).
- Data structures, algorithms, or external crates used.
- Error handling, caching strategies, and performance considerations.

## 4. Testing & Verification
- Unit and integration tests added in `tests/`.
- Edge cases, error scenarios, and boundary conditions tested.
- Local and CI validation results.

## 5. Notes & Future Recommendations
- Known limitations, tradeoffs, or potential future improvements.
- Important tips for subsequent agents interacting with this component.
```

---

## 📚 Index of Memory Documents

| File | Title | Summary |
|---|---|---|
| [`001-core-scanner-and-caching.md`](./001-core-scanner-and-caching.md) | Core Scanner, SQLite Cache & CLI | Initial scanner architecture, SQLite database caching, GitHub Tree API integration, metadata parsing, and interactive terminal list. |
| [`002-localhost-web-interface.md`](./002-localhost-web-interface.md) | Localhost Web Interface & Server | Embedded Single Page Application, Axum REST API endpoints, browser launcher, and interactive web explorer. |
| [`003-filter-and-similarity-skills.md`](./003-filter-and-similarity-skills.md) | Filter & Similarity Analysis | Heuristic similarity percentage calculation (Jaccard + Dice), text filtering, interactive REPL commands, and web similarity explorer. |
| [`004-shared-agent-memory.md`](./004-shared-agent-memory.md) | Shared Agent Memory System | Architecture and guidelines for the persistent `/memory` directory for AI agents and contributors. |
| [`005-cli-refinements-sqlite-and-cat-ui.md`](./005-cli-refinements-sqlite-and-cat-ui.md) | CLI Refinements, SQLite Persistence & Cat UI Redesign | Streamlined core commands (`scan`, `filter`, `similar`, `web`, `help`, `list`), SQLite cross-session listing, minimalist dark/light cat-themed web UI, and PR template. |
| [`006-cat-web-ui-redesign-and-enhancements.md`](./006-cat-web-ui-redesign-and-enhancements.md) | Enhanced Web UI Redesign with Cat Art & Details Modal | Full visual redesign from `skill-atlas-mm` featuring SVG cat animations (tail swish, blinking, eye tracking, hop cheer), center details modal with similar skills, query parser, and `/api/skills` endpoint. |
| [`007-cleanup-test-skills-and-data.md`](./007-cleanup-test-skills-and-data.md) | Exclusion & Cleanup of Acme Test Skills | Purged mock `acme` test fixture records from the persistent SQLite database and updated web UI filter placeholders. |
