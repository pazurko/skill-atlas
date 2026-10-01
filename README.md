# Skill Atlas ⚡

Skill Atlas is a high-performance interactive CLI built in Rust designed to scan GitHub repositories for AI agent skills and present them in a clean, terminal-rendered interactive menu with local SQLite caching. Easily navigate through discovered skills with arrow keys and open their GitHub definitions directly in your default browser.

---

## 🚀 Features

- **Blazing Fast GitHub Scanner**: Built natively in Rust for high throughput and minimal resource usage.
- **Local SQLite Caching**: Automatically saves scan results in a local SQLite database (`~/.skill-atlas/skills.db`). Subsequent scans verify whether the repository was updated on GitHub and reuse cached results to save bandwidth and API limits.
- **Repository Tree Analysis**: Loads the whole repository tree in one request and picks out skill files named `SKILL.md`, `skill.json` or `skill.yaml` at any depth (e.g. in `.agents/skills`, `.claude/skills`, `.junie/skills`, `skills/`). Supporting documents in skill folders and vendored folders such as `node_modules` or `.venv` are skipped.
- **Fast Downloads**: Skill files are fetched concurrently (up to 16 at a time) from `raw.githubusercontent.com`, which doesn't use up the GitHub API rate limit. A warning is shown if GitHub truncates the tree of a very large repository.
- **Automatic Metadata Extraction**: Parses skill names and first-sentence descriptions from YAML frontmatter, Markdown headings, or JSON/YAML configurations.
- **Interactive Terminal Menu**:
  - `↑` / `↓` (or `k` / `j`): Move selection cursor between discovered skills.
  - `Enter`: Open the selected skill's GitHub page in the default browser — the menu stays open so you can open more skills.
  - `s` / `S`: Inspect similarity heuristics and view the most similar counterpart skill with similarity percentage.
  - Opened skills are marked `✓ opened`, and the last opens stay listed below the menu while you navigate (`history` at the prompt shows the full audit of the session).
  - Skills sharing a name (e.g. one in `.claude/skills` and one shipped in a plugin) are all listed, with their full path shown so you can tell them apart.
  - `q` / `Esc` / `Ctrl+C`: Leave the menu and return to the `skill-atlas>` prompt.
- **Smart Filtering & Similarity Analysis**:
  - Filter results on the CLI (`--filter <QUERY>` or `filter [query]` in REPL) by name, description, or path.
  - Heuristic similarity detection (`similar` or `similar <target>` in REPL, `--similar` in CLI) computing percentage matches across skill names, descriptions, and file locations.
- **Persistent Interactive Session**: The CLI does not stop after one scan. From the `skill-atlas>` prompt you can scan other repositories, filter, discover similar skills, re-scan, list and open skills again, until you type `exit`.
- **Localhost Web Interface**: Launch an intuitive web UI via `skill-atlas web` or `web` inside the prompt to scan repositories, search/filter skills live, filter by format, discover similar skills with percentage badges, view cached history, and open definitions in your browser.
- **Flexible Repository Inputs**: Accepts `owner/repo`, full HTTPS URLs (`https://github.com/owner/repo`), and SSH URLs (`git@github.com:...`).
- **Non-Interactive & JSON Support**: Provides clean terminal output in non-TTY environments and a `--json` flag for scripting and CI pipelines.

---

## 📦 Installation

### Prerequisites
- [Rust toolchain (Cargo)](https://rustup.rs/) (edition 2021+)

### Build from Source

```bash
git clone https://github.com/pazurko/skill-atlas.git
cd skill-atlas
cargo build --release
```

The compiled binary will be available at `./target/release/skill-atlas`.

### Install to Cargo Bin Path

```bash
cargo install --path .
```

---

## 🛠️ Usage

### Scan a Repository

```bash
# Scan target repository directly
skill-atlas scan <githubrepo>

# Or launch the interactive session
skill-atlas
```

### Launch Web Interface

```bash
# Start localhost web interface and open in browser
skill-atlas web

# Customize host and port
skill-atlas web --host 127.0.0.1 --port 8080 --no-open
```

In a terminal, both forms end in a persistent `skill-atlas>` prompt (with `--json` or non-TTY output, `scan` stays one-shot):

```text
⚡ Welcome to Skill Atlas!
Type 'scan <githubrepo>' (e.g. scan https://github.com/JetBrains/kotlin), 'help' for commands, or 'exit' to quit.

skill-atlas> scan JetBrains/kotlin          # opens the arrow-key menu; q returns here
skill-atlas> open 3                         # open skill #3 of the last scan (or: open gradle)
skill-atlas> list                           # show the last results again
skill-atlas> https://github.com/owner/repo  # a bare repo also scans
skill-atlas> rescan                         # re-scan the last repo, bypassing the cache
skill-atlas> exit
```

| Command | Action |
| --- | --- |
| `scan <githubrepo> [-b <BRANCH>] [--refresh] [-f <QUERY>]` | Scan a repository (a bare `owner/repo` or URL works too) |
| `rescan` / `refresh` | Re-scan the last repository, bypassing the cache |
| `list` / `ls` | Show the skills of the last scan again (respecting active filter) |
| `filter [query\|clear]` / `f` | Filter current scan results by name, description, or path (or clear filter) |
| `similar [<number\|name>]` | Discover similar skills and duplicate definitions by percentage |
| `open <number\|name>` | Open a skill from the last scan in the browser |
| `history` / `opened` | Show every skill opened in this session (time, repo, name, URL) |
| `help` / `?` | Show available commands |
| `clear` | Clear the screen |
| `exit` / `quit` / `q` / `Ctrl+D` | Exit Skill Atlas |

#### Examples

```bash
# Using owner/repo shorthand
skill-atlas scan openai/swarm

# Using full GitHub URL
skill-atlas scan https://github.com/JetBrains/kotlin

# Output JSON
skill-atlas scan owner/repo --json

# Using GitHub token (to avoid rate limits)
skill-atlas scan owner/repo --token your_github_token
# ...or put GITHUB_TOKEN=... into a .env file in the current directory (loaded automatically)

# Bypass SQLite cache and force re-scan
skill-atlas scan owner/repo --no-cache

# Custom SQLite database location
skill-atlas scan owner/repo --db-path ./my-cache.db

# Specifying a branch
skill-atlas scan owner/repo --branch main
```

#### Visual Output Example

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

---

## ⌨️ Interactive Controls

| Key | Action |
| --- | --- |
| `↑` / `Up Arrow` or `k` | Navigate to previous skill |
| `↓` / `Down Arrow` or `j` | Navigate to next skill |
| `Enter` | Open the selected skill in your default web browser (menu stays open) |
| `q` / `Esc` or `Ctrl+C` | Return to the `skill-atlas>` prompt |

---

## 🧪 Testing

Run all unit and integration test suites:

```bash
cargo test
```

---

## 📖 Specifications & Agent Guidelines

- **CLI Specification**: [`spec/cli.md`](./spec/cli.md)
- **Agent Guidelines & Definition of Done**: [`agents.md`](./agents.md)

---

## 📄 License

MIT
