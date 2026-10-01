# Starred Skills & Organization Scanning

- **Status**: Implemented
- **Date**: 2026-10-01
- **Author/Agent**: Junie
- **PR / Issue Reference**: Starred Skills & Organization Scanning

---

## 1. Context & Motivation
- Users requested the ability to bookmark and star favorite or important agent skills discovered across repositories, with persistent SQLite storage and quick filtering/listing in the CLI, REPL, and Web UI.
- Users requested the ability to scan entire GitHub organizations (e.g. `org:JetBrains`, `org:openai`, or `https://github.com/JetBrains`), automatically discovering and scanning all public repositories belonging to that organization.

---

## 2. Requirements & Specification

### Starred Skills (Bookmarking)
- **SQLite Persistence**:
  - `starred_skills` table storing starred skill records with URLs, metadata, and timestamps.
  - Skills returned from SQLite cache or live scans have their `starred: bool` property populated.
  - Starred status survives repository re-scans, cache refreshes, and database reloads.
- **CLI & REPL**:
  - `skill-atlas starred` (aliases `star`, `stars`, `bookmarks`): Lists all starred skills from SQLite.
  - REPL commands:
    - `star <number|name>`: Stars the specified skill from current results.
    - `unstar <number|name>`: Unstars the specified skill.
    - `starred` / `stars` / `bookmarks`: Displays all starred skills in the interactive menu.
    - `filter status:starred` / `filter is:starred` / `filter starred`: Filters active skills to starred only.
  - Interactive Menu:
    - Key `*`, `t` (toggle), or `Space`: Toggles star status for the selected skill directly in the menu.
    - Visual rendering: `⭐` indicator next to starred skills and updated status line (`⭐ Starred '<name>'` / `☆ Unstarred '<name>'`).
- **Web UI & API**:
  - `POST /api/star` & `POST /api/skills/star`: Endpoint to toggle/set star status for a skill.
  - `GET /api/starred`: Endpoint returning all starred skills.
  - Web UI: Star toggle icon button on each skill card and in details modal, starred filter chip / toolbar button, and `status:starred` query filter support.

### Organization Scanning
- **Target Formats**:
  - `org:<org_name>` (e.g., `org:JetBrains`, `org:openai`).
  - GitHub Organization URLs (e.g., `https://github.com/JetBrains`, `github.com/openai`).
  - Shorthand `@<org_name>` or `org/<org_name>`.
- **Scanner Engine**:
  - Queries GitHub API (`/orgs/{org}/repos` with fallback to `/users/{user}/repos`) with pagination.
  - Discovers all public non-archived repositories for the organization.
  - Concurrently/sequentially scans each repository for skill files (`SKILL.md`, `skill.json`, `skill.yaml`, `skill.yml`).
  - Aggregates all discovered skills with proper repository URLs, branch references, and paths.
- **CLI / REPL / Web UI**:
  - Supports scanning organizations via `skill-atlas scan org:JetBrains`, in REPL `scan org:JetBrains`, and in the Web UI scan bar.

---

## 3. Architecture & Implementation Details
- `src/parser.rs`:
  - Added `ScanTarget` enum (`Repo`, `Org`) and `parse_scan_target` helper supporting `org:...`, GitHub org URLs, and repo identifiers.
- `src/storage.rs`:
  - Added `starred_skills` table schema and index.
  - Implemented `toggle_skill_starred`, `set_skill_starred`, `is_skill_starred`, and `get_starred_skills`.
  - Updated `get_cached_repository` and `get_all_cached_skills` with LEFT JOIN to populate `starred` flag.
- `src/scanner.rs`:
  - Added `starred: bool` field to `Skill` struct.
  - Implemented `fetch_org_repositories` and `scan_github_org` to retrieve org repos and scan them.
  - Updated `scan_github_repo` to populate `starred` state from SQLite.
- `src/interactive.rs`:
  - Added `MenuAction::ToggleStar` mapped to `*`, `t`, `T`, `Space`.
  - Updated menu renderer to display `⭐` for starred skills and updated key hints.
- `src/repl.rs`:
  - Added `ReplCommand::Star`, `ReplCommand::Unstar`, `ReplCommand::Starred`.
  - Added support for scanning organizations and filtering by starred status.
- `src/cli.rs`:
  - Added `Commands::Starred` and org scan target resolution.
- `src/web.rs` & `src/index.html`:
  - Added `/api/star` and `/api/starred` endpoints.
  - Added star toggle buttons, star icons, starred filter chip, and query parsing for `status:starred`.

---

## 4. Testing & Verification
- Unit and integration tests added in `tests/`:
  - `storage_test.rs`: Star toggle, persistence, list starred skills, survival across cache updates.
  - `parser_test.rs`: Org parsing (`org:JetBrains`, `https://github.com/JetBrains`, `@openai`, etc.).
  - `scanner_test.rs`: Org repo fetching with pagination and mock server.
  - `repl_test.rs`: REPL star/unstar/starred commands and org scanning.
  - `interactive_test.rs`: Star toggle menu key handling and status display.
  - `web_test.rs`: `/api/star`, `/api/starred`, and org scanning via Web API.
- Local validation with `cargo test`, `cargo fmt`, and `cargo clippy`.

---

## 5. Notes & Future Recommendations
- Rate limiting consideration: Scanning large organizations with dozens of repositories consumes GitHub API requests; recommend configuring `GITHUB_TOKEN` for large org scans.
