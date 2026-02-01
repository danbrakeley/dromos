# Dromos - Development Notes

## Open Design Questions

### Graph Traversal

- Direct neighbors only (requires chaining A→B→C→D)? ← current implementation
- Store strategic shortcut diffs for popular paths?
- Compute optimal paths on demand?

### Directionality

- One-way diffs (smaller storage, limited traversal)? ← current implementation
- Bidirectional diffs (doubles storage, simpler navigation)?

## Architectural Decision Records (ADRs)

Significant decisions are documented in `docs/decisions/` using the [MADR 4.0.0](https://adr.github.io/madr/) format.

### For Claude

- **Never create a new ADR without explicit human confirmation.** If a decision seems worth documenting, suggest it and wait for approval.
- ADR files are numbered sequentially: `NNNN-short-title.md` (e.g., `0001-use-bsdiff-for-patches.md`)
- Use `docs/decisions/adr-template.md` as the starting point for new ADRs
- When a decision is made during discussion, consider whether it warrants formal documentation—especially for:
  - Technology/library choices
  - Data format decisions
  - Architectural patterns
  - Trade-offs with long-term implications

## Decisions Made

See `docs/decisions/` for formal records. Informal/minor decisions can be noted here.

- **ROM hashing**: SHA-256 (see ADR-0002)
- **Diff format**: bsdiff (efficient for arbitrary binaries)
- **Database**: SQLite (source of truth) + petgraph StableGraph (in-memory cache rebuilt on startup)
- **Storage locations**: Platform-specific via `directories` crate (`%APPDATA%\dromos\data\` on Windows)
- **Diff storage**: Stored as files in `diffs/` directory (not database BLOBs)

## Conventions

- Module structure: `cli/`, `rom/`, `db/`, `graph/`, `storage/`, `diff/`
- Error handling: `thiserror` with `DromosError` enum in `error.rs`
- Hash display: First 16 hex chars for short display, full 64 for identification
