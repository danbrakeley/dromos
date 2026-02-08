use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DromosError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("Invalid NES file: {}", path.display())]
    InvalidNesFile { path: PathBuf },

    #[error("Unsupported ROM type: {extension}")]
    UnsupportedRomType { extension: String },

    #[error("ROM not found: {hash}")]
    RomNotFound { hash: String },

    #[error("ROM already exists: {hash}")]
    RomAlreadyExists { hash: String },

    #[error("Diff already exists between {0} and {1}")]
    DiffAlreadyExists(String, String),

    #[error("File not found: {}", path.display())]
    FileNotFound { path: PathBuf },

    #[error("Invalid hash format: {hash}")]
    InvalidHashFormat { hash: String },

    #[error("Diff creation failed: {0}")]
    DiffCreation(String),

    #[error("Diff application failed: {0}")]
    DiffApplication(String),

    #[error("No path from {from} to {to}")]
    NoPath { from: String, to: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Export error: {0}")]
    Export(String),

    #[error("Import error: {0}")]
    Import(String),
}

pub type Result<T> = std::result::Result<T, DromosError>;
