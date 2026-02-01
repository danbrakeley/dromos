use clap::Parser;
use std::process::ExitCode;

use dromos::cli::{Cli, Commands, RootRef};
use dromos::config::StorageConfig;
use dromos::rom::{format_hash, hash_rom_file};
use dromos::storage::StorageManager;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(cli: Cli) -> dromos::Result<()> {
    match cli.command {
        Commands::Hash { file } => {
            let metadata = hash_rom_file(&file)?;

            println!("Hash: {}", format_hash(&metadata.sha256));
            println!("Type: {}", metadata.rom_type);

            if let Some(header) = &metadata.nes_header {
                println!("PRG ROM: {} KB", header.prg_rom_size / 1024);
                println!("CHR ROM: {} KB", header.chr_rom_size / 1024);
                println!("Trainer: {}", if header.has_trainer { "Yes" } else { "No" });
            }

            Ok(())
        }

        Commands::AddRoot { file } => {
            let config = StorageConfig::default_paths()
                .ok_or_else(|| dromos::DromosError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine data directory",
                )))?;

            let mut storage = StorageManager::open(config)?;
            let metadata = storage.add_root(&file)?;

            println!("Added root ROM:");
            println!("  Hash: {}", format_hash(&metadata.sha256));
            println!("  Type: {}", metadata.rom_type);
            if let Some(name) = &metadata.filename {
                println!("  File: {}", name);
            }

            Ok(())
        }

        Commands::AddMod { root, mod_file } => {
            let config = StorageConfig::default_paths()
                .ok_or_else(|| dromos::DromosError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine data directory",
                )))?;

            let mut storage = StorageManager::open(config)?;
            let root_ref = RootRef::parse(&root);
            let metadata = storage.add_mod(root_ref, &mod_file)?;

            println!("Added mod ROM:");
            println!("  Hash: {}", format_hash(&metadata.sha256));
            println!("  Type: {}", metadata.rom_type);
            if let Some(name) = &metadata.filename {
                println!("  File: {}", name);
            }

            Ok(())
        }

        Commands::List => {
            let config = StorageConfig::default_paths()
                .ok_or_else(|| dromos::DromosError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine data directory",
                )))?;

            let storage = StorageManager::open(config)?;
            let (nodes, edges) = storage.list();

            if nodes.is_empty() {
                println!("No ROMs in database.");
                return Ok(());
            }

            println!("ROMs ({}):", nodes.len());
            for node in &nodes {
                let name = node.filename.as_deref().unwrap_or("<unnamed>");
                println!("  {} {} ({})", &format_hash(&node.sha256)[..16], name, node.rom_type);
            }

            if !edges.is_empty() {
                println!("\nEdges ({}):", edges.len());
                for (src, tgt, size) in &edges {
                    println!("  {}... -> {}... ({} bytes)", &src[..16], &tgt[..16], size);
                }
            }

            Ok(())
        }
    }
}
