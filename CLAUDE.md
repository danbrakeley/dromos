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

## Adding a New CLI Command

When adding a new command, modify these files in order:

### 1. `src/cli/commands.rs`
- Add variant to `Command` enum (e.g., `Rm { target: String }`)
- Add parsing in `Command::parse()` match arm
- Support aliases if needed (e.g., `"rm" | "remove"`)
- Return `Err("Usage: ...")` if required args missing

### 2. `src/cli/completer.rs`
- Add command name(s) to `ALL_COMMANDS` array
- If command takes file paths, add to `FILE_COMMANDS` array

### 3. `src/cli/repl.rs`
- Add match arm in `execute()` to call handler method
- Add help text in `print_help()`
- Implement `cmd_<name>()` handler method

### 4. `README.md`
- Update help output in usage example
- Move relevant item from TODO to DONE (if applicable)

### 5. Data layer (if command modifies state)

For database operations, add methods bottom-up:

1. **`src/db/repository.rs`** - Raw SQL operations (insert/select/delete)
2. **`src/graph/store.rs`** - In-memory graph operations
3. **`src/storage/manager.rs`** - High-level orchestration combining db + graph + filesystem
4. **`src/storage/mod.rs`** - Export any new public types

### Patterns to Follow

- **Hash resolution**: Use `find_node_by_hash_prefix()` to let users type partial hashes
- **Confirmation prompts**: For destructive ops, prompt `[y/N]` and check for `"y"` or `"yes"`
- **Output format**: `"Title  hash...  Type  [N links]"` for node listings
- **Error handling**: Return `Ok(())` after printing error with `eprintln!`, reserve `Err` for unexpected failures
- **Last added tracking**: Update `self.last_added` when adding nodes; clear it if removed

### Example: Minimal read-only command

```rust
// commands.rs - add variant
Foo { target: String },

// commands.rs - add parsing
"foo" => {
    if args.is_empty() {
        Err("Usage: foo <hash>".to_string())
    } else {
        Ok(Command::Foo { target: args[0].clone() })
    }
}

// repl.rs - add to execute()
Command::Foo { target } => self.cmd_foo(&target)?,

// repl.rs - implement handler
fn cmd_foo(&self, target: &str) -> Result<()> {
    let node = match self.storage.find_node_by_hash_prefix(target) {
        Some(n) => n,
        None => {
            eprintln!("ROM not found: {}", target);
            return Ok(());
        }
    };
    println!("Found: {}", node.title);
    Ok(())
}
```
