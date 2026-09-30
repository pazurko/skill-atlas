# Skill Atlas ⚡

Skill Atlas is a high-performance interactive CLI built in Rust designed to scan GitHub repositories for AI agent skills and present them in a clean, terminal-rendered interactive menu with local SQLite caching. Easily navigate through discovered skills with arrow keys and open their GitHub definitions directly in your default browser.

---

## 🚀 Features

- **Blazing Fast GitHub Scanner**: Built natively in Rust for high throughput and minimal resource usage.
- **Local SQLite Caching**: Automatically saves scan results in a local SQLite database (`~/.skill-atlas/skills.db`). Subsequent scans verify whether the repository was updated on GitHub and reuse cached results to save bandwidth and API limits.
- **Repository Tree Analysis**: Recursively identifies agent skill files (`SKILL.md`, `skill.json`, `skill.yaml`, `.agents/skills`, `.claude/skills`, `.junie/skills`, `skills/`, etc.).
- **Automatic Metadata Extraction**: Parses skill names and first-sentence descriptions from YAML frontmatter, Markdown headings, or JSON/YAML configurations.
- **Interactive Terminal Menu**:
  - `↑` / `↓` (or `k` / `j`): Move selection cursor between discovered skills.
  - `Enter`: Open the selected skill's GitHub page directly in the default browser.
  - `q` / `Esc` / `Ctrl+C`: Exit the CLI.
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

# Or launch interactively
skill-atlas
```

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

↑/↓ navigate • Enter open in GitHub • q quit
```

---

## ⌨️ Interactive Controls

| Key | Action |
| --- | --- |
| `↑` / `Up Arrow` or `k` | Navigate to previous skill |
| `↓` / `Down Arrow` or `j` | Navigate to next skill |
| `Enter` | Open the selected skill in your default web browser |
| `q` / `Esc` or `Ctrl+C` | Exit the CLI |

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
