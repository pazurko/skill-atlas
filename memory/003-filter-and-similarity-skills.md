# Filter and Similarity Analysis

- **Status**: Implemented
- **Date**: 2026-10-01
- **Author/Agent**: Core Development Team
- **PR / Issue Reference**: PR #7 (`feature/filter-and-similar-skills`)

---

## 1. Context & Motivation
Large repositories often contain dozens or hundreds of agent skills with duplicate definitions, overlapping functionality, or slight naming variations across plugins or subdirectories. Users needed instant filtering and heuristic similarity metrics to detect duplicates and related skills.

## 2. Requirements & Specification
- **Filter**:
  - Filter results across name, description, and file path.
  - CLI flag `-f, --filter <QUERY>` in `scan` command.
  - REPL command `filter [query|clear]` / `f` to filter the current scan results.
  - Web UI live search input and format chips (`All Formats`, `SKILL.md`, `YAML`, `JSON`).
- **Similarity**:
  - Heuristic similarity percentage calculation (0.0% to 100.0%) comparing skill name, description keywords, and directory paths.
  - Combined Jaccard set similarity and character bigram Dice coefficient for robust fuzzy matching.
  - CLI flag `--similar` and `--min-similarity <PERCENT>`.
  - REPL command `similar [<number|name>]` and interactive menu shortcut `s` / `S`.
  - Web interface `/api/similar` REST endpoints and dedicated `⚡ Similar Pairs` exploration panel.

## 3. Architecture & Implementation Details
- `src/similarity.rs`:
  - Tokenization with stop-word filtering (`tokenize`).
  - Set Jaccard index (`jaccard_similarity`) and character bigram Dice coefficient (`dice_bigram_similarity`).
  - Weighted composite similarity calculation (`calculate_similarity`).
  - `find_similar_skills`, `find_all_similar_pairs`, and `filter_skills`.
- `src/cli.rs`: Added CLI options `--filter`, `--similar`, and `--min-similarity`.
- `src/repl.rs`: Added `filter` and `similar` command handlers in the persistent interactive prompt.
- `src/interactive.rs`: Added `s` key handler to inspect the most similar skill from the interactive menu.
- `src/web.rs`: Added `POST /api/similar` and `GET /api/similar` endpoints with `SimilarPayload` and `SimilarResponse`.
- `src/index.html`: Enhanced web frontend with live filtering, format chips, and similarity cards.

## 4. Testing & Verification
- Unit tests in `tests/similarity_test.rs`:
  - Tokenization, stop-word elimination, Jaccard and Dice coefficients.
  - Similarity percentage calculation for exact matches, partial matches, and distinct skills.
  - Target similarity search and all-pairs similarity detection.
- Integration tests in `tests/repl_test.rs` and `tests/web_test.rs`:
  - Verified `filter` and `similar` in REPL sessions.
  - Verified `/api/similar` HTTP requests and JSON responses.

## 5. Notes & Future Recommendations
- Threshold defaults to 30.0% across CLI/web, which balances recall and precision for skill definitions.
