---
status: accepted
date: 2026-01-31
---

# Use SQLite for Persistence with Petgraph for In-Memory Graph

## Context and Problem Statement

Dromos needs to store ROM metadata and the relationships (diffs) between them. The data forms a graph structure where nodes are ROMs and edges are diffs. How should we persist this data and enable efficient graph traversal?

## Decision Drivers

- Need durable storage that survives restarts
- Need efficient graph traversal for pathfinding between ROMs
- Should work offline without external services
- Must be embeddable (no separate database server)
- Should handle moderate dataset sizes (thousands of ROMs, not millions)

## Considered Options

- SQLite only (with recursive CTEs for graph queries)
- Petgraph only (serialize to JSON/bincode)
- SQLite + petgraph hybrid
- Embedded graph database (e.g., sled + custom graph layer)

## Decision Outcome

Chosen option: "SQLite + petgraph hybrid", because it combines the durability and query flexibility of SQLite with the efficient in-memory graph algorithms of petgraph.

### Consequences

- Good, because SQLite provides ACID transactions and proven durability
- Good, because petgraph provides O(1) node lookup and efficient traversal algorithms
- Good, because SQLite is the source of truth, making debugging and data recovery straightforward
- Good, because graph is rebuilt on startup, ensuring consistency
- Bad, because startup time increases linearly with dataset size (acceptable for expected scale)
- Bad, because memory usage scales with graph size (acceptable for expected scale)

## Pros and Cons of the Options

### SQLite only

Use SQLite with recursive CTEs for graph traversal queries.

- Good, because single storage layer, simpler architecture
- Good, because no memory overhead for graph structure
- Bad, because recursive CTEs are complex and harder to optimize
- Bad, because graph algorithms (shortest path, etc.) would need reimplementation in SQL

### Petgraph only

Serialize the petgraph structure directly to disk (JSON, bincode, etc.).

- Good, because simpler code, single data structure
- Good, because fast graph operations
- Bad, because no query flexibility (can't easily filter or search)
- Bad, because corruption recovery is difficult
- Bad, because concurrent access is problematic

### SQLite + petgraph hybrid

SQLite as source of truth, petgraph rebuilt from database on startup.

- Good, because best of both worlds: durability + performance
- Good, because can use SQL for ad-hoc queries and reporting
- Good, because petgraph's `StableGraph` keeps indices valid after node removal
- Neutral, because requires keeping two representations in sync
- Bad, because startup cost to rebuild graph

### Embedded graph database

Use a dedicated graph storage solution.

- Good, because purpose-built for graph operations
- Bad, because adds significant dependency complexity
- Bad, because less mature ecosystem than SQLite
- Bad, because overkill for expected dataset size

## More Information

Implementation details:

- `StorageManager` owns both the SQLite `Connection` and the `RomGraph`
- On startup, `load_graph_from_db()` rebuilds the petgraph from SQLite
- All mutations go through `StorageManager` which updates both SQLite and petgraph atomically
- `StableGraph` is used so that `NodeIndex` values remain valid after deletions
