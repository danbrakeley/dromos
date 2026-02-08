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

## Data Revision System

During development, data storage format may change in backwards-incompatible ways. The `DATA_REVISION` constant in `src/db/schema.rs` tracks this. When you increment it:

1. Increment `DATA_REVISION` in `src/db/schema.rs`
2. Collapse all migrations into `migrations/001_initial.sql`
3. Delete migration files 002+
4. Update the `Migrations::new(vec![...])` to only include 001

On next startup, dromos will detect the revision change and automatically wipe the database and diffs.

## Conventions

- Module structure: `cli/`, `rom/`, `db/`, `graph/`, `storage/`, `diff/`, `exchange/`
- Error handling: `thiserror` with `DromosError` enum in `error.rs`
- Hash display: First 16 hex chars for short display, full 64 for identification
- Title display: Use `format_display_title(title, version)` to show `"Title [version]"` consistently
- Colorized output: Use `theme::` functions from `src/cli/theme.rs` (respects `NO_COLOR` and TTY detection)

## Colorized Output

All terminal colors are handled through `src/cli/theme.rs`. This module respects the `NO_COLOR` environment variable and TTY detection.

### Available Theme Functions

| Function         | Color             | Use For                                  |
| ---------------- | ----------------- | ---------------------------------------- |
| **Semantic**     |                   |                                          |
| `error()`        | red               | Error messages                           |
| `warning()`      | yellow            | Warnings                                 |
| `success()`      | green             | Success confirmations                    |
| `info()`         | cyan              | Informational messages                   |
| **Data Display** |                   |                                          |
| `title()`        | bright white      | ROM titles                               |
| `label()`        | yellow            | Categorical labels (ROM type)            |
| `meta()`         | cyan              | Secondary metadata (version, link count) |
| **Chrome**       |                   |                                          |
| `prompt()`       | bright blue, bold | Input prompts                            |
| `dim()`          | dark grey         | De-emphasized text                       |
| `header()`       | bold              | Section headers                          |
| **Helpers**      |                   |                                          |
| `styled_hash()`  | blue + dark blue  | Hash with "..." suffix                   |
| `print_banner()` | —                 | Startup logo with version/build date     |

## Key Data Structures

Understanding these three structs and their relationships is essential:

| Struct         | Location           | Purpose                                                      |
| -------------- | ------------------ | ------------------------------------------------------------ |
| `RomNode`      | `graph/store.rs`   | In-memory graph node (title, version, hash, rom_type)        |
| `NodeRow`      | `db/repository.rs` | Full database row (all fields including NES header metadata) |
| `NodeMetadata` | `db/repository.rs` | User-editable fields for add/edit operations                 |

**Data flow:**

- `NodeMetadata` → user input during add/edit
- `NodeMetadata` + `RomMetadata` → `Repository::insert_node()` → database
- Database → `Repository::load_all_nodes()` → `NodeRow` → `RomNode` (in-memory graph)

**When to use each:**

- Need to display a ROM? Use `RomNode` from graph (fast, in-memory)
- Need full metadata (NES header, description, tags)? Use `NodeRow` from `get_node_row_by_hash()`
- Adding/editing a ROM? Build a `NodeMetadata` from user prompts

## Adding a New Field to ROM Nodes

When adding a new field (like `version`, `tags`, etc.):

### 1. Database migration (`migrations/NNN_name.sql`)

```sql
ALTER TABLE nodes ADD COLUMN field_name TEXT;
```

### 2. Update `src/db/schema.rs`

Add migration to the `Migrations::new(vec![...])` list.

### 3. Update `src/db/repository.rs`

- Add field to `NodeRow` struct
- Add field to `NodeMetadata` struct (if user-editable)
- Update `map_row_to_node_row()` to read the new column
- Update ALL SELECT queries (4 places: `get_node_by_hash`, `get_node_by_id`, `load_all_nodes`, and column comments)
- Update `insert_node()` to write the new column
- Update `update_node_metadata()` if field is editable

### 4. Update `src/graph/store.rs` (if field needed in memory)

- Add field to `RomNode` struct
- Update test helper `make_node()`

### 5. Update `src/storage/manager.rs`

- Update `load_graph_from_db()` to include field when creating `RomNode`
- Update `add_node()` to include field when creating `RomNode`
- Update `update_node_metadata()` to sync field to graph if needed
- Update test helper `add_node_from_metadata()`

### 6. Update `src/cli/repl.rs` (if user-facing)

- Add prompting in `prompt_metadata()` and `prompt_metadata_from_row()`
- Update `AddResult` and `LastAdded` structs if field affects display
- Update display locations to show the new field

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
- **Output format**: `"Title [version]  hash...  Type  [N links]"` for node listings
- **Title display**: Always use `format_display_title(&node.title, node.version.as_deref())` for consistent output
- **Error handling**: Return `Ok(())` after printing error with `eprintln!("{}", theme::error("message"))`, reserve `Err` for unexpected failures
- **Last added tracking**: Update `self.last_added` when adding nodes; clear it if removed
- **Adding ROMs**: Use `ensure_rom_added()` helper - handles existence check, metadata prompting, and database insertion

### Key Helper Functions in `repl.rs`

| Function                                       | Purpose                                                             |
| ---------------------------------------------- | ------------------------------------------------------------------- |
| `ensure_rom_added(file, rl)`                   | Add ROM if not exists, prompt for full metadata, return `AddResult` |
| `format_display_title(title, version)`         | Format title with optional version: `"Title [1.0]"` or `"Title"`    |
| `prompt_metadata(rl, default_title, existing)` | Prompt for all metadata fields (new ROM)                            |
| `prompt_metadata_from_row(rl, node_row)`       | Prompt for all metadata fields (editing existing ROM)               |
| `prompt_with_initial(rl, prompt, initial)`     | Single-line prompt with editable default                            |

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
            eprintln!("{}", theme::error(&format!("ROM not found: {}", target)));
            return Ok(());
        }
    };
    let display = format_display_title(&node.title, node.version.as_deref());
    println!("Found: {}", theme::title(&display));
    Ok(())
}
```

## Export/Import Format

The `exchange/` module handles portable export folders for sharing ROM collections.

### Folder Structure

```text
my-export/
├── index.json
└── diffs/
    ├── abcdef01_12345678.bsdiff
    └── ...
```

### `index.json` Schema

```json
{
  "dromos_export": { "version": 1, "data_revision": 2, "exported_at": "..." },
  "files": [{ "sha256": "...", "title": "...", "rom_type": "NES", ... }],
  "diffs": [{ "source_sha256": "...", "target_sha256": "...", "diff_path": "...", "diff_size": 1234, "sha256": "..." }]
}
```

- `files`: array of ROM node metadata
- `diffs`: array of diff edges; each entry includes a `sha256` field with the hex-encoded SHA-256 hash of the `.bsdiff` file for integrity verification

### Module Layout (`src/exchange/`)

| File        | Purpose                                                      |
| ----------- | ------------------------------------------------------------ |
| `mod.rs`    | Module declarations and re-exports                           |
| `format.rs` | Serde structs (`ExportManifest`, `ExportNode`, `ExportEdge`) |
| `export.rs` | `write_folder()` — writes folder from DB/graph data          |
| `import.rs` | `analyze_import()` + `execute_import()` — two-phase import   |

### Import Flow

1. **Analyze**: Parse folder's `index.json`, compare nodes against local DB, identify conflicts (differing metadata fields)
2. **Prompt**: Show conflicts to user, ask whether to overwrite
3. **Execute**: Insert new nodes, optionally overwrite conflicts, insert edges (skip duplicates), copy diff files (with SHA-256 verification)

## Testing

### Test Helpers

Each module has test helpers for creating test data:

- `db/repository.rs`: `make_metadata(hash_byte, filename)`, `make_node_metadata(title)`
- `graph/store.rs`: `make_node(db_id, hash_byte, title)`, `make_edge(db_id, diff_path)`
- `storage/manager.rs`: `make_metadata(hash_byte, filename)`, `StorageManager::new_in_memory(temp_dir)`, `add_node_from_metadata(metadata, title)`

### Running Tests

```bash
cargo test                    # Run all tests
cargo test db::               # Run only db module tests
cargo test --lib              # Skip doc tests
cargo test test_name          # Run specific test
```
