---
status: accepted
date: 2026-01-31
decision-makers: [human]
---

# Use Rust for Implementation

## Context and Problem Statement

Dromos is a CLI application for managing ROM images through a graph of binary diffs. It needs to:

- Target Windows, Linux, and macOS
- Search local filesystems quickly
- Hash files efficiently
- Read/write graph data structures
- Generate and apply binary diffs (using bsdiff)

Which programming language should we use?

## Decision Drivers

- Cross-platform support with minimal friction
- Single binary deployment (no runtime dependencies)
- Performance for binary operations and file hashing
- Quality of CLI tooling ecosystem
- Developer familiarity (willing to learn)

## Considered Options

- Rust
- Go
- Zig
- C/C++
- TypeScript

## Decision Outcome

Chosen option: "Rust", because:

- Excellent CLI ecosystem (clap, walkdir, etc.)
- Natural fit for binary manipulation and low-level operations
- Single binary output with easy cross-compilation
- Strong library support: bsdiff-rs for diffs, petgraph for graphs, sha2/md5 for hashing
- Memory safety without garbage collection

### Consequences

- Good, because single binary deployment simplifies distribution
- Good, because mature ecosystem for all required operations
- Good, because strong type system catches errors at compile time
- Bad, because steeper learning curve than Go
- Bad, because slower compilation times

## Pros and Cons of the Options

### Rust

- Good, because excellent CLI tooling (clap is best-in-class)
- Good, because binary manipulation is natural and safe
- Good, because bsdiff-rs library available
- Good, because petgraph provides mature graph data structures
- Good, because cross-compilation is well-supported
- Bad, because learning curve for ownership/borrowing
- Bad, because slower compile times

### Go

- Good, because already familiar to developer
- Good, because trivial cross-compilation
- Good, because fast compilation
- Good, because single binary output
- Bad, because binary diff libraries require CGO or pure-Go reimplementations
- Bad, because less elegant for low-level binary operations

### Zig

- Good, because C-level control with modern tooling
- Good, because can directly use C libraries (bsdiff)
- Good, because excellent cross-compilation
- Neutral, because learning while building
- Bad, because smaller ecosystem, fewer libraries

### C/C++

- Good, because maximum performance and control
- Good, because bsdiff is natively C
- Bad, because cross-platform CLI requires more build system work
- Bad, because manual memory management increases development cost

### TypeScript

- Good, because familiar to developer
- Bad, because requires runtime (Node/Deno/Bun) or bundling
- Bad, because binary operations need native modules or WASM
- Bad, because not ideal for performance-critical file operations
