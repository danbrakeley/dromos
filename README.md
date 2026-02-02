# Dromos

A command-line tool for managing ROM images through a graph of binary diffs.

## Concept

Dromos stores relationships between ROM files rather than the files themselves. The database maintains a graph where:

- **Nodes** represent ROM files (identified by hash)
- **Edges** represent binary diffs between ROMs

Users supply a ROM they already possess, and Dromos shows which related ROMs are reachable. Selecting a target applies the necessary diff(s) to generate it.

## Use Cases

- Navigate ROM hack family trees (base game -> translation -> bug fixes -> enhancements)
- Distribute patches rather than copyrighted content
- Efficiently store many variants of similar ROMs

## Usage

Dromos runs as an interactive shell:

```bash
$ dromos
dromos> help
Commands:
  add <file>              Add a ROM to the database
  build <source> <hash>   Build a ROM by applying diffs from source to target
  edit <hash>             Edit metadata for a ROM
  link <file1> [file2]    Create bidirectional links between ROMs
  links <file|hash>       Show all links for a ROM
  list, ls                List all ROMs (sorted by title)
  rm, remove <hash>       Remove a ROM and all its links
  search <query>          Search ROMs by title
  hash <file>             Show ROM hash without adding to database
  help                    Show this help
  quit, exit              Exit dromos

dromos> add "Super Game (USA).nes"
Adding file Super Game (USA).nes
Title: Super Game
Source URL:
Version: USA, Rev 0
Release Date (YYYY-MM-DD): 1999-01-01
Tags (comma-separated): platformer
Description (press Enter to skip):
Added: Super Game [USA, Rev 0] (abc12345...)

dromos> link "Super Game (USA).nes" "Super Game (USA) [PRG1].nes"
Adding file Super Game (USA) [PRG1].nes
Title: Super Game
Source URL:
Version: USA, Rev 1
Release Date (YYYY-MM-DD): 1999-01-01
Tags (comma-separated): platformer
Description (press Enter to skip):
Added: Super Game [USA, Rev 1] (c32154ba...)
Linked: Super Game [USA, Rev 0] <-> Super Game [USA, Rev 1]

dromos> list
Super Game [USA, Rev 0]  abc12345...  NES  [1 link]
Super Game [USA, Rev 1]  c32154ba...  NES  [1 link]

dromos> links abc12345
Super Game [USA, Rev 0]  (abc12345...)
  -> Super Game [USA, Rev 1]  (1.2 KB)

dromos> quit
```

## Development

```bash
cargo build          # Build debug version
cargo build --release # Build optimized release version
cargo run            # Build and run
cargo test           # Run tests
cargo clippy         # Run linter
cargo fmt            # Format code
```

## TODO

- more metadata: author, author_url
- colorized output
- build requires a starting rom; should we support storing that in the database?

## DONE

- Version displayed in brackets after title throughout the interface (e.g., "Super Mario Bros [1.0]")
- Edit command to modify metadata for existing ROMs
- ROM metadata fields: source URL, version, release date, tags, and multi-line description
- Build a ROM from source file to target hash using `build` command with BFS pathfinding
- Remove a ROM node and all its links with `rm` command (with confirmation)
- Show link counts in list, view links for a ROM with `links` command
- Tab completion for commands and file paths
- Interactive REPL with command history
- Add and link ROMs with bidirectional diffs
- Search ROMs by title
- Generate first graph connection with diff
- Parse NES header and only hash actual ROM data
- Read file and generate hash
