# Dromos - Development Notes

## Open Design Questions

### Diff Format

- IPS/BPS (ROM hacking standard, BPS has checksums)
- bsdiff/xdelta (more efficient for arbitrary binaries)
- Custom delta encoding?

### Graph Traversal

- Direct neighbors only (requires chaining A→B→C→D)?
- Store strategic shortcut diffs for popular paths?
- Compute optimal paths on demand?

### Directionality

- One-way diffs (smaller storage, limited traversal)?
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

## Conventions

(To be established as development progresses)
