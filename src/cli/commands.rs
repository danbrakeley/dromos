use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dromos")]
#[command(about = "ROM image management through binary diffs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Hash a ROM file and display its metadata
    Hash {
        /// Path to the ROM file
        file: PathBuf,
    },

    /// Add a root ROM to the database
    AddRoot {
        /// Path to the ROM file
        file: PathBuf,
    },

    /// Add a modified ROM as a child of an existing root
    AddMod {
        /// Root ROM identifier (SHA-256 hash or filename)
        root: String,

        /// Path to the modified ROM file
        mod_file: PathBuf,
    },

    /// List all ROMs and their relationships
    List,
}

/// Represents a reference to a root ROM, either by hash or file path
#[derive(Debug, Clone)]
pub enum RootRef {
    Hash([u8; 32]),
    File(PathBuf),
}

impl RootRef {
    pub fn parse(s: &str) -> RootRef {
        // If it's a 64-character hex string, treat it as a hash
        if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Some(hash) = crate::rom::parse_hash(s) {
                return RootRef::Hash(hash);
            }
        }
        RootRef::File(PathBuf::from(s))
    }
}
