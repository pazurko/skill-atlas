# Core Scanner, SQLite Cache & CLI

- **Status**: Implemented
- **Date**: 2026-10-01
- **Author/Agent**: Core Development Team
- **PR / Issue Reference**: Initial Core Implementation

---

## 1. Context & Motivation
Developers and AI agents need a friction-free way to discover AI agent skill definitions across GitHub repositories without cloning large repositories or manually browsing through complex directory trees.

## 2. Requirements & Specification
- Accept various repository identifier formats: `owner/repo`, HTTPS URLs, SSH git URLs.
- Scan repository trees via the GitHub Trees API (`git/trees/<branch>?recursive=1`) and identify valid skill files (`SKILL.md`, `skill.json`, `skill.yaml`, `skill.yml`) at any depth.
- Concurrently download skill file contents via `raw.githubusercontent.com` (falling back to GitHub Blobs API) to avoid rate limits.
- Parse metadata (skill names and descriptions) from YAML frontmatter, Markdown headers, or JSON/YAML payloads.
- Local SQLite caching (`~/.skill-atlas/skills.db`) checking commit SHAs before re-fetching remote files.
- Interactive terminal menu (Crossterm) with keyboard navigation (`↑`/`↓`, `Enter` to open in browser, `q` to return) and persistent `skill-atlas>` prompt.

## 3. Architecture & Implementation Details
- `src/parser.rs`: Parsing and validation of GitHub repository identifiers and URLs.
- `src/scanner.rs`: Asynchronous GitHub repository scanner using Reqwest and Tokio; concurrent raw downloads (up to 16 workers).
- `src/metadata.rs`: Skill metadata extraction and first-sentence summarizer.
- `src/storage.rs`: Rusqlite SQLite cache for repositories and skill definitions.
- `src/interactive.rs`: Interactive terminal menu using Crossterm raw mode.
- `src/repl.rs`: Interactive session prompt supporting `scan`, `rescan`, `list`, `open`, `history`, `help`, `clear`, `exit`.
- `src/history.rs`: Audit log tracking browser open events during the interactive session.
- `src/dotenv.rs`: Automatic environment variable resolution from `.env` files.

## 4. Testing & Verification
- Unit and integration tests in `tests/`: `cli_test.rs`, `scanner_test.rs`, `storage_test.rs`, `parser_test.rs`, `metadata_test.rs`, `dotenv_test.rs`, `interactive_test.rs`, `repl_test.rs`, `fixture_repo_test.rs`.
- Validated with mock GitHub servers (Wiremock) and local file fixtures.

## 5. Notes & Future Recommendations
- Ensure GitHub rate limiting is handled cleanly when no token is provided.
- Large repositories where the tree API returns `"truncated": true` report a warning banner to the user.
