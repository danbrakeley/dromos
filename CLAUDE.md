# Dromos - Development Notes

## Open Design Questions

### ROM Identification
- Which hash algorithm(s)? CRC32, MD5, SHA1, or multiple?
- How to handle headered vs headerless ROMs?
- Should regional variants be separate nodes or metadata on a single node?

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

## Decisions Made

(None yet)

## Conventions

(To be established as development progresses)
