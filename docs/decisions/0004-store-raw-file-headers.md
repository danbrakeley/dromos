---
status: accepted
date: 2025-02-04
---

# Store Raw File Headers Instead of Parsed Fields

## Context and Problem Statement

Dromos needs to store file header metadata (e.g., NES iNES headers) to reconstruct ROM files accurately. The current approach parses header fields into separate database columns, then reconstructs the header when building ROMs.

Should dromos continue storing parsed header fields, or store the raw header bytes directly?

## Decision Drivers

- NES headers have multiple format variants (archaic iNES, iNES 0.7, iNES 1.0, NES 2.0) with subtle differences
- As dromos adds more file types in the future, they will also have lots of history and nuances to deal with, and the database will either need more and more columns, or a new solution to store all the different header fields.
- Becoming a master of various header formats seems like a distraction at the moment, and not critical to dromos' core goals.
- Storing just the raw bytes leaves open the possibility to parse the headers in the future, should that become important.

## Considered Options

- Store parsed header fields (current approach)
- Store raw header bytes

## Decision Outcome

Chosen option: "Store raw header bytes", because it guarantees byte-identical reconstruction, eliminates parsing/encoding edge cases, and establishes a reusable pattern for future file types.

### Consequences

- Good, because built ROMs are guaranteed byte-identical to originals
- Good, because implementation is simpler (one BLOB column vs 12+ typed columns)
- Good, because no risk of data corruption from parsing/encoding bugs
- Good, because pattern extends naturally to other file types (store their headers the same way)
- Good, because parsed fields can still be computed on-demand for display
- Neutral, because searching by header fields requires parsing at query time (or adding indexed columns later)
- Bad, because existing parsed columns become redundant (kept for backwards compatibility)
