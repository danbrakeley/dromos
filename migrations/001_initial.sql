CREATE TABLE nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256 TEXT NOT NULL UNIQUE,
    filename TEXT,
    rom_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    prg_rom_size INTEGER,
    chr_rom_size INTEGER,
    has_trainer INTEGER
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

CREATE INDEX idx_nodes_sha256 ON nodes(sha256);
CREATE INDEX idx_edges_source ON edges(source_id);
CREATE INDEX idx_edges_target ON edges(target_id);
