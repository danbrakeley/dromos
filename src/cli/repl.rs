use std::io::{self, Write};
use std::path::Path;

use crate::config::StorageConfig;
use crate::error::Result;
use crate::graph::RomNode;
use crate::rom::{format_hash, hash_rom_file};
use crate::storage::StorageManager;

use super::Command;

pub struct ReplState {
    pub storage: StorageManager,
    pub last_added: Option<LastAdded>,
}

#[derive(Clone)]
pub struct LastAdded {
    pub hash: [u8; 32],
    pub title: String,
}

impl ReplState {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let storage = StorageManager::open(config)?;
        Ok(ReplState {
            storage,
            last_added: None,
        })
    }

    pub fn execute(&mut self, cmd: Command) -> Result<bool> {
        match cmd {
            Command::Quit => return Ok(false),
            Command::Help => self.print_help(),
            Command::Hash { file } => self.cmd_hash(&file)?,
            Command::Add { file } => self.cmd_add(&file)?,
            Command::Link { files } => self.cmd_link(&files)?,
            Command::List => self.cmd_list(),
            Command::Search { query } => self.cmd_search(&query),
        }
        Ok(true)
    }

    fn print_help(&self) {
        println!("Commands:");
        println!("  add <file>              Add a ROM to the database");
        println!("  link <file1> [file2]    Create bidirectional links between ROMs");
        println!("  list, ls                List all ROMs (sorted by title)");
        println!("  search <query>          Search ROMs by title");
        println!("  hash <file>             Show ROM hash without adding to database");
        println!("  help                    Show this help");
        println!("  quit, exit              Exit dromos");
    }

    fn cmd_hash(&self, file: &Path) -> Result<()> {
        let metadata = hash_rom_file(file)?;

        println!("Hash: {}", format_hash(&metadata.sha256));
        println!("Type: {}", metadata.rom_type);

        if let Some(header) = &metadata.nes_header {
            println!("PRG ROM: {} KB", header.prg_rom_size / 1024);
            println!("CHR ROM: {} KB", header.chr_rom_size / 1024);
            println!(
                "Trainer: {}",
                if header.has_trainer { "Yes" } else { "No" }
            );
        }

        Ok(())
    }

    fn cmd_add(&mut self, file: &Path) -> Result<()> {
        // Check if file exists
        if !file.exists() {
            eprintln!("File not found: {}", file.display());
            return Ok(());
        }

        // Hash the file first to check if it already exists
        let metadata = hash_rom_file(file)?;
        if self.storage.node_exists(&metadata.sha256) {
            let node = self.storage.get_node_by_hash(&metadata.sha256).unwrap();
            println!(
                "ROM already exists: {} ({}...)",
                node.title,
                &format_hash(&metadata.sha256)[..16]
            );
            return Ok(());
        }

        // Get default title from filename
        let default_title = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Prompt for title
        let title = prompt_with_default("Title", &default_title)?;

        // Add to database
        let metadata = self.storage.add_node(file, &title)?;

        println!(
            "Added: {} ({}...)",
            title,
            &format_hash(&metadata.sha256)[..16]
        );

        // Update last added
        self.last_added = Some(LastAdded {
            hash: metadata.sha256,
            title,
        });

        Ok(())
    }

    fn cmd_link(&mut self, files: &[std::path::PathBuf]) -> Result<()> {
        match files.len() {
            1 => self.link_to_last(&files[0]),
            2 => self.link_two_files(&files[0], &files[1]),
            _ => {
                eprintln!("Usage: link <file1> [file2]");
                Ok(())
            }
        }
    }

    fn link_to_last(&mut self, file: &Path) -> Result<()> {
        let last = match &self.last_added {
            Some(last) => last.clone(),
            None => {
                eprintln!("No previous ROM to link to. Use 'link <file1> <file2>' instead.");
                return Ok(());
            }
        };

        // Confirm link to last added
        let prompt = format!("Link to \"{}\"? [Y/n]", last.title);
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" || input == "no" {
            println!("Cancelled.");
            return Ok(());
        }

        // Check if file exists
        if !file.exists() {
            eprintln!("File not found: {}", file.display());
            return Ok(());
        }

        // Hash and check if already exists
        let metadata = hash_rom_file(file)?;
        let needs_add = !self.storage.node_exists(&metadata.sha256);

        let title = if needs_add {
            // Prompt for title
            let default_title = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let title = prompt_with_default("Title", &default_title)?;

            // Add the new ROM
            self.storage.add_node(file, &title)?;
            println!(
                "Added: {} ({}...)",
                title,
                &format_hash(&metadata.sha256)[..16]
            );
            title
        } else {
            let node = self.storage.get_node_by_hash(&metadata.sha256).unwrap();
            node.title.clone()
        };

        // Need to find the file for the last_added ROM
        // For now, require the user to have the file accessible
        // This is a limitation - we'd need to store original file paths
        eprintln!(
            "Note: To create links, you need both ROM files. Use 'link <file1> <file2>' with both files."
        );

        // Update last added
        self.last_added = Some(LastAdded {
            hash: metadata.sha256,
            title,
        });

        Ok(())
    }

    fn link_two_files(&mut self, file_a: &Path, file_b: &Path) -> Result<()> {
        // Check both files exist
        if !file_a.exists() {
            eprintln!("File not found: {}", file_a.display());
            return Ok(());
        }
        if !file_b.exists() {
            eprintln!("File not found: {}", file_b.display());
            return Ok(());
        }

        // Hash both files
        let metadata_a = hash_rom_file(file_a)?;
        let metadata_b = hash_rom_file(file_b)?;

        // Add first file if needed
        let title_a = if !self.storage.node_exists(&metadata_a.sha256) {
            let default_title = file_a
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let prompt_msg = format!("Title for {}", file_a.display());
            let title = prompt_with_default(&prompt_msg, &default_title)?;
            self.storage.add_node(file_a, &title)?;
            println!(
                "Added: {} ({}...)",
                title,
                &format_hash(&metadata_a.sha256)[..16]
            );
            title
        } else {
            let node = self.storage.get_node_by_hash(&metadata_a.sha256).unwrap();
            node.title.clone()
        };

        // Add second file if needed
        let title_b = if !self.storage.node_exists(&metadata_b.sha256) {
            let default_title = file_b
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let prompt_msg = format!("Title for {}", file_b.display());
            let title = prompt_with_default(&prompt_msg, &default_title)?;
            self.storage.add_node(file_b, &title)?;
            println!(
                "Added: {} ({}...)",
                title,
                &format_hash(&metadata_b.sha256)[..16]
            );
            title
        } else {
            let node = self.storage.get_node_by_hash(&metadata_b.sha256).unwrap();
            node.title.clone()
        };

        // Create bidirectional links
        self.storage.link_nodes(file_a, file_b)?;
        println!("Linked: {} <-> {}", title_a, title_b);

        // Update last added to the second file
        self.last_added = Some(LastAdded {
            hash: metadata_b.sha256,
            title: title_b,
        });

        Ok(())
    }

    fn cmd_list(&self) {
        let (nodes, edges) = self.storage.list();

        if nodes.is_empty() {
            println!("No ROMs in database.");
            return;
        }

        // Sort by title
        let mut sorted_nodes: Vec<&RomNode> = nodes.clone();
        sorted_nodes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

        for node in sorted_nodes {
            println!(
                "{}  {}...  {}",
                node.title,
                &format_hash(&node.sha256)[..16],
                node.rom_type
            );
        }

        if !edges.is_empty() {
            println!("\nLinks: {}", edges.len());
        }
    }

    fn cmd_search(&self, query: &str) {
        let (nodes, _) = self.storage.list();
        let query_lower = query.to_lowercase();

        let matches: Vec<&RomNode> = nodes
            .into_iter()
            .filter(|n| n.title.to_lowercase().contains(&query_lower))
            .collect();

        if matches.is_empty() {
            println!("No matches found for \"{}\"", query);
            return;
        }

        for node in matches {
            println!(
                "{}  {}...  {}",
                node.title,
                &format_hash(&node.sha256)[..16],
                node.rom_type
            );
        }
    }
}

/// Prompt the user with a default value.
fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    Ok(if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    })
}
