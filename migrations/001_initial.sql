CREATE TABLE nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256 TEXT NOT NULL UNIQUE,
    filename TEXT,
    title TEXT,
    rom_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    source_url TEXT,
    version TEXT,
    release_date TEXT,
    tags TEXT,
    description TEXT,
    source_file_header BLOB
);

CREATE TABLE edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES nodes(id),
    target_id INTEGER NOT NULL REFERENCES nodes(id),
    diff_path TEXT NOT NULL,
    diff_size INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_id, target_id)
);

CREATE TABLE dromos_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX idx_nodes_sha256 ON nodes(sha256);
CREATE INDEX idx_edges_source ON edges(source_id);
CREATE INDEX idx_edges_target ON edges(target_id);
