# CLI Specification: Skill Atlas

## Overview
Skill Atlas is an interactive CLI designed to maximize developer efficiency and minimize search friction by scanning GitHub repositories for agent skills and presenting them in an interactive, actionable list with local SQLite caching.

## Core Command & Usage

```bash
# Scan a repository for AI agent skills
skill-atlas scan <githubrepo>

# List all cached skills from local SQLite database
skill-atlas list

# Filter cached skills by keyword query
skill-atlas filter <query>

# Discover similar skills and duplicate definitions
skill-atlas similar [target]

# Start localhost web interface
skill-atlas web

# Interactive session (persistent `skill-atlas>` prompt when launched without arguments)
skill-atlas
```

### Core Commands & Arguments
- `scan <githubrepo>`: Target repository identifier or URL (e.g., `owner/repo`, `https://github.com/owner/repo`, or `git@github.com:...`). If omitted in an interactive terminal, the interactive session is started. If omitted in a non-interactive environment, a usage message is printed.
- `list` (alias `ls`): Load and display all cached skills across all repositories stored in the local SQLite database (`~/.skill-atlas/skills.db`).
- `filter [query]` (alias `f`): Filter skills by keyword query across name, description, or file path.
- `similar [target]`: Find similar skills based on heuristics and similarity percentages (threshold >= 30.0%). Without arguments, lists all detected similar pairs. With a target skill, lists all skills similar to that target.
- `web` (alias `serve`): Start the localhost web interface on port 3000 (`-p, --port <PORT>`).
- Environment variables (`GITHUB_TOKEN`, `GH_TOKEN`) are automatically loaded from `.env` files to prevent API rate limiting.

### Web Interface & UI
- Accessible via `skill-atlas web` or `web` inside the interactive prompt.
- Warm ginger-cat aesthetic with dark & light theme switcher and animated cat motifs (blinking eyes, swishing tail, interactive mouse-following eye pupils, hop cheers, and strolling footer cat).
- Instant query filter (supporting `repo:`, `name:`, `desc:`, `path:`, `status:` and phrases with keyword highlighting), sortable table columns, center peek modal dialog with similar skills exploration, and "Show all saved" SQLite catalogue viewer.

## Output Example & Visual Presentation

When running `skill-atlas scan <githubrepo>`, the interactive menu renders structured cards with repository badges, numbered indices, first-sentence descriptions, and file badges:

```text
$ skill-atlas scan https://github.com/JetBrains/kotlin
JetBrains/kotlin 6 skills

 [ 1] › analysis-api-create-cherry-pick-issue                                       SKILL.md ↗
      Create a KTIJ cherry-pick tracking issue for a KT fix that needs to be cherry...

 [ 2] › analysis-api-mark-internal-apis                                             SKILL.md ↗
      Drive the per-module internal-API codebase test, then refine the suggested an...

 [ 3] › build-bump-gradle-version                                                   SKILL.md ↗
      Bumps the Gradle wrapper/distribution version that BUILDS the Kotlin project...

 [ 4] › build-tools-bump-gradle-api                                                 SKILL.md ↗
      Bumps the Gradle API version that Kotlin Gradle plugins are compiled against...

 [ 5] › build-tools-bump-gradle-in-tests                                            SKILL.md ↗
      Bump the maximum supported Gradle version in the `kotlin-gradle-plugin-integr...

 [ 6] › minimize-repro-for-diagnostic-test                                         SKILL.md ↗
      Makes a minimal reproduction of a Frontend-related bug as a diagnostic test...

↑/↓ navigate • Enter open in GitHub • s similar • q back to prompt
```

## Workflow & Behavior

1. **Local SQLite Cache Verification**:
   - Checks local SQLite database for prior scan results matching `(owner, repo, branch)`.
   - Checks if the remote repository has been updated since the last scan (by querying the latest commit SHA).
   - If unchanged, retrieves the cached skills directly from SQLite without re-scanning the GitHub tree or fetching file contents.

2. **Repository Scan & Metadata Extraction**:
   - If the repository has updated or is scanned for the first time, loads the whole repository tree in a single request (`git/trees/<branch>?recursive=1`, no depth limit) and selects skill definitions:
     - Only files named `SKILL.md`, `skill.json`, `skill.yaml` or `skill.yml` (any letter case, at any depth, e.g. under `.agents/skills`, `.claude/skills`, `.junie/skills`, `skills/`) are skills. Other files inside skill folders (e.g. `references/*.md`) are supporting documents and are neither downloaded nor listed.
     - Files inside vendored, generated or VCS folders (`.git`, `.hg`, `.svn`, `node_modules`, `.venv`, `venv`, `__pycache__`, `.tox`, `.mypy_cache`, `.pytest_cache`, `site-packages`) are ignored. Only whole folder names match (`my-venv-tools/` is not ignored).
     - Only `blob` entries count (a folder that is itself named `SKILL.md` is not a skill). Folder names with spaces, unicode or mixed case are supported, and skills shipped inside plugins/resources (e.g. `plugins/<plugin>/resources/.../skills/<skill>/SKILL.md`) are found like any other.
   - Skill files are downloaded concurrently (at most 16 at a time) from `raw.githubusercontent.com` at the scanned commit, which does not count against the GitHub API rate limit; if that fails, the GitHub Blobs API is used. Results are sorted by name (case-insensitive), then by path.
   - **Duplicate names**: several skills with the same name (e.g. one in `.claude/skills` and one shipped in a plugin) are all kept. In the menu and plain list their file badge shows the full path instead of just the file name, so they can be told apart.
   - If GitHub reports the tree as truncated (very large repositories), the scan still returns the skills found and prints `⚠️  Warning: GitHub truncated the repository tree for this large repository; results may be incomplete.`; `--json` output contains `"truncated": true`.
   - Extracts and cleans metadata (skill name, first sentence of description, file path, GitHub URL) across YAML frontmatter, JSON, YAML files, and Markdown headings.
   - Saves the fresh scan results and commit SHA into the local SQLite database.
   - **GitHub URLs** have the form `https://github.com/<owner>/<repo>/blob/<ref>/<path>`. With an explicit `--branch`, `<ref>` is that branch. With the default `HEAD`, `<ref>` is the repository's default branch as reported by the GitHub API (e.g. `master` for `JetBrains/kotlin`, never an assumed `main`); if it cannot be determined, `HEAD` is used (GitHub resolves it to the default branch). URLs of cached results are rebuilt the same way when loaded, so stale links from older caches are corrected.

3. **Interactive Selection List**:
   - Outputs the discovered skills in a clean, terminal-rendered interactive menu displaying skill names, paths, and descriptions.
   - **Navigation**:
     - `↑` / `↓` (or `k` / `j`): Move selection cursor between identified skills.
   - **Action**:
     - `Enter`: Open the selected skill's GitHub definition in the default web browser. The menu stays open and shows a status line, so several skills can be opened in a row. The status line is kept while navigating (it only changes on the next open).
     - `s` / `S`: Show similarity information and the most similar counterpart skill with similarity percentage for the currently selected skill.
   - **Opened history (audit)**: every open attempt (from the menu or `open`) is recorded for the whole session, across repositories, with local time (`HH:MM:SS`), `owner/repo`, skill name and URL; failed browser launches are recorded as `✗ ... (failed: <error>)`. Skills opened successfully are marked `✓ opened` in the menu, and the menu shows the 5 most recent entries below the key hints under `Opened in this session (N, 'history' at the prompt shows all):`. A one-shot `scan` outside a session keeps the history only while the menu is open.
     - `q` / `Esc` / `Ctrl+C`: Leave the menu and return to the `skill-atlas>` prompt.

4. **Persistent Interactive Session (`skill-atlas>` prompt)**:
   - Started in an interactive terminal (stdin and stdout are TTYs) when:
     - `skill-atlas` is run without arguments, or `skill-atlas scan` without a repository (the session starts empty);
     - `skill-atlas scan <githubrepo>` is run without `--json` (the repository is scanned first, then the session continues).
   - `--json` output and non-TTY environments stay one-shot: results are printed once and the process exits (exit code 1 on scan errors).
   - The session never ends because of a scan: after a scan, leaving the menu, or an error, the prompt is shown again.
   - `--token`, `--branch`, `--no-cache` and `--db-path` given on the command line apply to every scan in the session.
   - **Commands** (case-insensitive command word):

     | Command | Behavior |
     | --- | --- |
     | `scan <githubrepo> [-b\|--branch <BRANCH>] [--refresh\|--no-cache] [-f\|--filter <QUERY>]` | Scan a repository and show the interactive menu. A bare repository (contains `/` or starts with `git@`) is treated as `scan <repo>`. Missing repository, a missing branch value, extra repositories or unknown options print `Usage: scan <githubrepo> [--branch <BRANCH>] [--refresh] [--filter <QUERY>]`. |
     | `rescan` / `refresh` | Re-scan the last successfully scanned repository (same branch), bypassing the cache. Without a previous scan: `Nothing to rescan yet. Run 'scan <githubrepo>' first.` |
     | `list` / `ls` | Show the skills of the last scan again in the menu (respecting any active filter). |
     | `filter [query\|clear]` / `f` | Filter current scan results across name, description, or path. Running without arguments or `filter clear` resets the filter. |
     | `similar [<number\|name>]` | Find and report similar skills based on heuristics and similarity percentages (threshold >= 30%). Without arguments, lists all detected similar pairs. With a skill index or name, lists all skills similar to that target. |
     | `open <number\|name>` | Open a skill of the last scan in the browser: 1-based index, else exact case-insensitive name, else first name containing the query. When the matched name is shared by several skills, the first (by path) is opened and `Note: <n> skills are named '<name>'; opened <path>. Use 'open <number>' to pick another.` is printed. Out-of-range index: `Invalid index. Please choose between 1 and N.`; no match: `Skill matching '<query>' not found in recent results.`; no argument: `Usage: open <number\|name>`. |
     | `history` / `opened` | List every skill opened in this session, oldest first, under `Opened in this session (N):`. Empty: `Nothing opened yet in this session.` |
     | `web` / `serve` | Start the localhost web interface and open it in the default browser (`web [--port <PORT>] [--no-open]`). |
     | `help` / `?` | List available commands. |
     | `clear` / `cls` | Clear the terminal screen. |
     | `exit` / `quit` / `q` | Print `Goodbye!` and exit. End of input (`Ctrl+D`) also exits. |

   - Empty lines are ignored; any other input prints `Unknown command: '<cmd>'. Type 'help' for available commands.`
   - `list`, `filter`, `similar`, and `open` before any skills were found print `No skills listed yet in this session. Run 'scan <githubrepo>' first.`
   - Scan errors (not found, rate limit, invalid identifier, network) are printed as `❌ Error: ...` and the session continues; the results of the previous successful scan are kept.
   - A repository with no skills prints `No agent skills found in <owner/repo>.` and clears the previous results.
   - A failure to launch the browser is reported (`Failed to open browser: ...`) without ending the session.

5. **Localhost Web Interface**:
   - Launched using `skill-atlas web` (or `skill-atlas serve`) from the terminal or `web` / `serve` within the interactive prompt.
   - Binds an HTTP server to the configured host and port (default: `127.0.0.1:3000`) and opens the default browser unless `--no-open` is specified.
   - **Frontend GUI Capabilities**:
     - Modern single-page application embedded in the binary.
     - **Repository Scan Form**: Input fields for repository identifier / URL (`owner/repo`, `https://github.com/...`, `git@github.com:...`), branch ref, optional GitHub token, and cache bypass toggle.
     - **Results View**: Displays repository badges, total skills count, branch, commit SHA, and cache state indicators (`📦 SQLite Cache` vs `⚡ Fresh Scan`).
     - **Real-time Filter**: Instant search filter by skill name, description, or file path with item counter.
     - **Format Filter Chips**: Quick filtering by file type (`All Formats`, `SKILL.md`, `YAML`, `JSON`).
     - **Similarity Explorer**:
       - `⚡ Similar Pairs` panel showing all detected duplicate and similar skills with similarity percentage badges.
       - Per-card `⚡ Similar` button to focus on skills similar to a specific skill with percentage match badges.
     - **Skill Cards**: Numbered cards displaying skill name, file badge (`SKILL.md ↗`, `skill.yaml ↗`, or disambiguated full path), clean description, copyable path, similarity badge (when active), and direct link to open the definition on GitHub.
     - **Cached History Sidebar**: Lists previously scanned repositories with quick-click reloading from local SQLite cache.
   - **REST API Endpoints**:
     - `GET /`: Serves the embedded HTML/CSS/JavaScript web interface.
     - `GET /api/skills[?q=<query>]`: Retrieves all cached skills from local SQLite database with optional filter query.
     - `POST /api/scan`: Executes scan for repository or repositories specified in JSON payload `{"repo": "owner/repo", "branch": "HEAD", "refresh": false, "token": "...", "filter": "..."}` or `{"targets": [...]}`.
     - `GET /api/scan?repo=...&branch=...&refresh=...&filter=...`: Executes scan via query parameters.
     - `POST /api/similar`: Returns similar pairs and target skill matches for repository specified in JSON payload `{"repo": "owner/repo", "target": "...", "min_similarity": 30.0}`.
     - `GET /api/similar?repo=...&target=...&min_similarity=...`: Returns similarity analysis via query parameters.
     - `GET /api/cached`: Returns JSON list of cached repositories.
     - `GET /api/health`: Returns health status and application version.
