# Memory Specification: Shared Agent Memory

## Overview
Skill Atlas maintains a structured shared memory repository under `/memory` to store long-term context, architectural decisions, and operational history across multiple AI agents and contributor sessions.

## Directory Layout

```text
memory/
├── README.md                           # Main index and template instructions
├── 001-core-scanner-and-caching.md     # Memory for core scanner & SQLite cache
├── 002-localhost-web-interface.md      # Memory for localhost web interface
├── 003-filter-and-similarity-skills.md # Memory for filter and similarity analysis
├── 004-shared-agent-memory.md          # Memory for shared agent memory system
└── NNN-<feature-or-request>.md         # Sequential memory files for future features/requests
```

## Agent Requirements

1. **Read Memory on Task Start**: AI agents must inspect `memory/README.md` and any relevant memory documents before modifying code or planning architecture.
2. **Create or Update Memory on Task Completion**:
   - For every new feature or significant request, agents must generate a new memory document using sequential numeric prefixing (`NNN-<feature-name>.md`).
   - If an existing feature is modified or refactored, agents must update the corresponding memory document.
   - Update the table of contents / index in `memory/README.md`.
3. **Standard Structure**:
   - Title & Metadata Header (Status, Date, Author/Agent, PR/Issue Reference)
   - 1. Context & Motivation
   - 2. Requirements & Specification
   - 3. Architecture & Implementation Details
   - 4. Testing & Verification
   - 5. Notes & Future Recommendations
