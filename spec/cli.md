# CLI Specification: Skill Atlas

## Overview
Skill Atlas is an interactive CLI designed to maximize developer efficiency and minimize search friction by scanning GitHub repositories for agent skills and presenting them in an interactive, actionable list with local SQLite caching.

## Core Command & Usage

```bash
# Direct scanning via subcommand
skill-atlas scan <githubrepo> [OPTIONS]

# Interactive session (persistent `skill-atlas>` prompt when launched without arguments)
skill-atlas
```

### Arguments & Options
- `<githubrepo>`: Target repository identifier or URL (e.g., `owner/repo`, `https://github.com/owner/repo`, or `git@github.com:...`). If omitted in an interactive terminal, the interactive session is started. If omitted in a non-interactive environment, a usage message is printed to stderr.
- `-t, --token <TOKEN>`: GitHub personal access token to prevent API rate limiting. If omitted, `GITHUB_TOKEN` (then `GH_TOKEN`) from the environment is used. At startup, `KEY=VALUE` lines from a `.env` file in the current directory, and then from the nearest `.env` in the executable's directory or its ancestors (e.g. the project root for `target/release/skill-atlas`, so a binary started from Finder or another directory still finds it), are loaded into the environment (blank lines and `#` comments skipped, `export ` prefix and quotes stripped); variables already set in the environment are never overridden, and a missing `.env` is ignored. Without a token GitHub allows only 60 API requests per hour per IP.
- `-b, --branch <BRANCH>`: Target Git branch or ref (default: `HEAD`).
- `--json`: Output raw JSON scan results instead of interactive menu.
- `--no-cache` (alias: `--refresh`): Bypass local SQLite cache and re-scan GitHub directly.
- `--db-path <PATH>`: Custom SQLite database path (default: `~/.skill-atlas/skills.db`).

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

↑/↓ navigate • Enter open in GitHub • q back to prompt
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
   - Skill files are downloaded concurrently (at most 16 at a time) from `raw.githubusercontent.com` at the scanned commit, which does not count against the GitHub API rate limit; if that fails, the GitHub Blobs API is used. Results are sorted by name.
   - If GitHub reports the tree as truncated (very large repositories), the scan still returns the skills found and prints `⚠️  Warning: GitHub truncated the repository tree for this large repository; results may be incomplete.`; `--json` output contains `"truncated": true`.
   - Extracts and cleans metadata (skill name, first sentence of description, file path, GitHub URL) across YAML frontmatter, JSON, YAML files, and Markdown headings.
   - Saves the fresh scan results and commit SHA into the local SQLite database.
   - **GitHub URLs** have the form `https://github.com/<owner>/<repo>/blob/<ref>/<path>`. With an explicit `--branch`, `<ref>` is that branch. With the default `HEAD`, `<ref>` is the repository's default branch as reported by the GitHub API (e.g. `master` for `JetBrains/kotlin`, never an assumed `main`); if it cannot be determined, `HEAD` is used (GitHub resolves it to the default branch). URLs of cached results are rebuilt the same way when loaded, so stale links from older caches are corrected.

3. **Interactive Selection List**:
   - Outputs the discovered skills in a clean, terminal-rendered interactive menu displaying skill names, paths, and descriptions.
   - **Navigation**:
     - `↑` / `↓` (or `k` / `j`): Move selection cursor between identified skills.
   - **Action**:
     - `Enter`: Open the selected skill's GitHub definition in the default web browser. The menu stays open and shows a status line, so several skills can be opened in a row.
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
     | `scan <githubrepo> [-b\|--branch <BRANCH>] [--refresh\|--no-cache]` | Scan a repository and show the interactive menu. A bare repository (contains `/` or starts with `git@`) is treated as `scan <repo>`. Missing repository, a missing branch value, extra repositories or unknown options print `Usage: scan <githubrepo> [--branch <BRANCH>] [--refresh]`. |
     | `rescan` / `refresh` | Re-scan the last successfully scanned repository (same branch), bypassing the cache. Without a previous scan: `Nothing to rescan yet. Run 'scan <githubrepo>' first.` |
     | `list` / `ls` | Show the skills of the last scan again in the menu. |
     | `open <number\|name>` | Open a skill of the last scan in the browser: 1-based index, else exact case-insensitive name, else first name containing the query. Out-of-range index: `Invalid index. Please choose between 1 and N.`; no match: `Skill matching '<query>' not found in recent results.`; no argument: `Usage: open <number\|name>`. |
     | `help` / `?` | List available commands. |
     | `clear` / `cls` | Clear the terminal screen. |
     | `exit` / `quit` / `q` | Print `Goodbye!` and exit. End of input (`Ctrl+D`) also exits. |

   - Empty lines are ignored; any other input prints `Unknown command: '<cmd>'. Type 'help' for available commands.`
   - `list` and `open` before any skills were found print `No skills listed yet in this session. Run 'scan <githubrepo>' first.`
   - Scan errors (not found, rate limit, invalid identifier, network) are printed as `❌ Error: ...` and the session continues; the results of the previous successful scan are kept.
   - A repository with no skills prints `No agent skills found in <owner/repo>.` and clears the previous results.
   - A failure to launch the browser is reported (`Failed to open browser: ...`) without ending the session.
