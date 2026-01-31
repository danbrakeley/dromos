# Dromos

A command-line tool for managing ROM images through a graph of binary diffs.

## Concept

Dromos stores relationships between ROM files rather than the files themselves. The database maintains a graph where:

- **Nodes** represent ROM files (identified by hash)
- **Edges** represent binary diffs between ROMs

Users supply a ROM they already possess, and Dromos shows which related ROMs are reachable. Selecting a target applies the necessary diff(s) to generate it.

## Use Cases

- Navigate ROM hack family trees (base game → translation → bug fixes → enhancements)
- Distribute patches rather than copyrighted content
- Efficiently store many variants of similar ROMs

## Status

Early design phase.

## Development

```bash
cargo build          # Build debug version
cargo build --release # Build optimized release version
cargo run            # Build and run
cargo test           # Run tests
cargo clippy         # Run linter
cargo fmt            # Format code
```
