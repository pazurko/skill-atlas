# CLI Specification: Skill Atlas

## Overview
Skill Atlas is an interactive CLI designed to maximize developer efficiency and minimize search friction by scanning GitHub repositories for agent skills and presenting them in an interactive, actionable list with local SQLite caching.

## Core Command & Usage

```bash
# Direct scanning via subcommand
skill-atlas scan <githubrepo> [OPTIONS]

# Interactive mode (prompts for repository when launched directly or without arguments)
skill-atlas
```

### Arguments & Options
- `<githubrepo>`: Target repository identifier or URL (e.g., `owner/repo`, `https://github.com/owner/repo`, or `git@github.com:...`). If omitted in an interactive terminal, the user is prompted to enter a repository.
- `-t, --token <TOKEN>`: GitHub personal access token to prevent API rate limiting.
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

↑/↓ navigate • Enter open in GitHub • q quit
```

## Workflow & Behavior

1. **Local SQLite Cache Verification**:
   - Checks local SQLite database for prior scan results matching `(owner, repo, branch)`.
   - Checks if the remote repository has been updated since the last scan (by querying the latest commit SHA).
   - If unchanged, retrieves the cached skills directly from SQLite without re-scanning the GitHub tree or fetching file contents.

2. **Repository Scan & Metadata Extraction**:
   - If the repository has updated or is scanned for the first time, analyzes the repository tree for skill definitions (`SKILL.md`, `skill.json`, `skill.yaml`, `.agents/skills`, `.claude/skills`, `.junie/skills`, `skills/`).
   - Extracts and cleans metadata (skill name, first sentence of description, file path, GitHub URL) across YAML frontmatter, JSON, YAML files, and Markdown headings.
   - Saves the fresh scan results and commit SHA into the local SQLite database.

3. **Interactive Selection List**:
   - Outputs the discovered skills in a clean, terminal-rendered interactive menu displaying skill names, paths, and descriptions.
   - **Navigation**:
     - `↑` / `↓` (or `k` / `j`): Move selection cursor between identified skills.
   - **Action**:
     - `Enter`: Open the selected skill's GitHub definition directly in the default web browser.
     - `q` / `Esc` / `Ctrl+C`: Exit the interactive menu.
