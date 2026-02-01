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
  link <file1> [file2]    Create bidirectional links between ROMs
  links <file|hash>       Show all links for a ROM
  list, ls                List all ROMs (sorted by title)
  rm, remove <hash>       Remove a ROM and all its links
  search <query>          Search ROMs by title
  hash <file>             Show ROM hash without adding to database
  help                    Show this help
  quit, exit              Exit dromos

dromos> add "Super Mario Bros 2 (PRG0).nes"
Title [Super Mario Bros 2 (PRG0).nes]: Super Mario Bros 2 (US)
Added: Super Mario Bros 2 (US) (cba920f9...)

dromos> link "Super Mario Bros 2 (PRG0).nes" "Super Mario Bros 2 (PRG1).nes"
Title for Super Mario Bros 2 (PRG1).nes [Super Mario Bros 2 (PRG1).nes]: Super Mario Bros 2 (US, PRG1)
Added: Super Mario Bros 2 (US, PRG1) (728d0ca6...)
Linked: Super Mario Bros 2 (US) <-> Super Mario Bros 2 (US, PRG1)

dromos> list
Super Mario Bros 2 (US)        cba920f9...  NES  [1 link]
Super Mario Bros 2 (US, PRG1)  728d0ca6...  NES  [1 link]

dromos> links cba920f9
Super Mario Bros 2 (US)  (cba920f9...)
  -> Super Mario Bros 2 (US, PRG1)  (1.2 KB)

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

- derive a rom file by using a starting rom file a db link
- colorized output
- unit tests: where are they and how robust are they?

## DONE

- Remove a ROM node and all its links with `rm` command (with confirmation)
- Show link counts in list, view links for a ROM with `links` command
- Tab completion for commands and file paths
- Interactive REPL with command history
- Add and link ROMs with bidirectional diffs
- Search ROMs by title
- Generate first graph connection with diff
- Parse NES header and only hash actual ROM data
- Read file and generate hash
