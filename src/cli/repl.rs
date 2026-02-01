use std::io::{self, Write};
use std::path::Path;

use rustyline::history::DefaultHistory;
use rustyline::Editor;

use crate::config::StorageConfig;
use crate::error::Result;
use crate::graph::RomNode;
use crate::rom::{format_hash, hash_rom_file};
use crate::storage::StorageManager;

use super::completer::DromosHelper;
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

    pub fn execute(&mut self, cmd: Command, rl: &mut Editor<DromosHelper, DefaultHistory>) -> Result<bool> {
        match cmd {
            Command::Quit => return Ok(false),
            Command::Help => self.print_help(),
            Command::Hash { file } => self.cmd_hash(&file)?,
            Command::Add { file } => self.cmd_add(&file, rl)?,
            Command::Link { files } => self.cmd_link(&files, rl)?,
            Command::Links { target } => self.cmd_links(&target)?,
            Command::List => self.cmd_list(),
            Command::Rm { target } => self.cmd_rm(&target)?,
            Command::Search { query } => self.cmd_search(&query),
        }
        Ok(true)
    }

    fn print_help(&self) {
        println!("Commands:");
        println!("  add <file>              Add a ROM to the database");
        println!("  link <file1> [file2]    Create bidirectional links between ROMs");
        println!("  links <file|hash>       Show all links for a ROM");
        println!("  list, ls                List all ROMs (sorted by title)");
        println!("  rm, remove <hash>       Remove a ROM and all its links");
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

    fn cmd_add(&mut self, file: &Path, rl: &mut Editor<DromosHelper, DefaultHistory>) -> Result<()> {
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

        // Get default title from filename (without extension)
        let default_title = title_from_filename(file);

        // Prompt for title with editable default
        let title = prompt_with_initial(rl, "Title", &default_title)?;

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

    fn cmd_link(&mut self, files: &[std::path::PathBuf], rl: &mut Editor<DromosHelper, DefaultHistory>) -> Result<()> {
        match files.len() {
            1 => self.link_to_last(&files[0], rl),
            2 => self.link_two_files(&files[0], &files[1], rl),
            _ => {
                eprintln!("Usage: link <file1> [file2]");
                Ok(())
            }
        }
    }

    fn link_to_last(&mut self, file: &Path, rl: &mut Editor<DromosHelper, DefaultHistory>) -> Result<()> {
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
            // Prompt for title with editable default
            let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            println!("Adding file {}", filename);
            let default_title = title_from_filename(file);
            let title = prompt_with_initial(rl, "Title", &default_title)?;

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

    fn link_two_files(&mut self, file_a: &Path, file_b: &Path, rl: &mut Editor<DromosHelper, DefaultHistory>) -> Result<()> {
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
            let filename = file_a.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            println!("Adding file {}", filename);
            let default_title = title_from_filename(file_a);
            let title = prompt_with_initial(rl, "Title", &default_title)?;
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
            let filename = file_b.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            println!("Adding file {}", filename);
            let default_title = title_from_filename(file_b);
            let title = prompt_with_initial(rl, "Title", &default_title)?;
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
        let (nodes, _edges) = self.storage.list();

        if nodes.is_empty() {
            println!("No ROMs in database.");
            return;
        }

        // Sort by title
        let mut sorted_nodes: Vec<&RomNode> = nodes.clone();
        sorted_nodes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

        for node in sorted_nodes {
            let link_count = self.storage.link_count(&node.sha256);
            let link_info = if link_count > 0 {
                format!("  [{} link{}]", link_count, if link_count == 1 { "" } else { "s" })
            } else {
                String::new()
            };
            println!(
                "{}  {}...  {}{}",
                node.title,
                &format_hash(&node.sha256)[..16],
                node.rom_type,
                link_info
            );
        }
    }

    fn cmd_links(&self, target: &str) -> Result<()> {
        // Try to find node: first as file, then as hash prefix
        let node = if std::path::Path::new(target).exists() {
            // It's a file path - hash it and look up
            let metadata = hash_rom_file(std::path::Path::new(target))?;
            self.storage.get_node_by_hash(&metadata.sha256)
        } else {
            // Try as hash prefix
            self.storage.find_node_by_hash_prefix(target)
        };

        let node = match node {
            Some(n) => n,
            None => {
                eprintln!("ROM not found: {}", target);
                return Ok(());
            }
        };

        let neighbors = self.storage.get_neighbors(&node.sha256);

        println!("{}  ({}...)", node.title, &format_hash(&node.sha256)[..16]);

        match neighbors {
            Some(links) if !links.is_empty() => {
                for (neighbor, diff_size) in links {
                    println!(
                        "  -> {}  ({})",
                        neighbor.title,
                        format_size(diff_size)
                    );
                }
            }
            _ => {
                println!("  (no links)");
            }
        }

        Ok(())
    }

    fn cmd_rm(&mut self, target: &str) -> Result<()> {
        // Try to find node by hash prefix
        let node = self.storage.find_node_by_hash_prefix(target);

        let node = match node {
            Some(n) => n,
            None => {
                eprintln!("ROM not found: {}", target);
                return Ok(());
            }
        };

        let sha256 = node.sha256;
        let title = node.title.clone();
        let link_count = self.storage.link_count(&sha256);

        // Prompt for confirmation
        let link_text = if link_count == 1 { "link" } else { "links" };
        print!("Remove '{}' and {} {}? [y/N]: ", title, link_count, link_text);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Cancelled.");
            return Ok(());
        }

        // Perform the removal
        let result = self.storage.remove_node(&sha256)?;

        println!(
            "Removed: {} ({} edge{}, {} diff file{})",
            result.title,
            result.edges_removed,
            if result.edges_removed == 1 { "" } else { "s" },
            result.diff_files_removed,
            if result.diff_files_removed == 1 { "" } else { "s" }
        );

        // Clear last_added if it was the removed node
        if let Some(ref last) = self.last_added
            && last.hash == sha256
        {
            self.last_added = None;
        }

        Ok(())
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

/// Prompt the user with an editable initial value using rustyline.
fn prompt_with_initial(rl: &mut Editor<DromosHelper, DefaultHistory>, prompt: &str, initial: &str) -> Result<String> {
    let prompt_str = format!("{}: ", prompt);
    match rl.readline_with_initial(&prompt_str, (initial, "")) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                Ok(initial.to_string())
            } else {
                Ok(trimmed.to_string())
            }
        }
        Err(_) => Ok(initial.to_string()),
    }
}

/// Known ROM file extensions to strip from titles.
const ROM_EXTENSIONS: &[&str] = &[
    ".nes", ".smc", ".sfc", ".gb", ".gbc", ".gba", ".nds", ".n64", ".z64", ".v64",
    ".gen", ".md", ".sms", ".gg", ".pce", ".bin", ".iso", ".cue", ".zip", ".7z",
];

/// Extract a title from a filename, stripping known ROM extensions.
fn title_from_filename(path: &Path) -> String {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let lower = filename.to_lowercase();
    for ext in ROM_EXTENSIONS {
        if lower.ends_with(ext) {
            return filename[..filename.len() - ext.len()].to_string();
        }
    }

    filename.to_string()
}

/// Format a byte size in a human-readable way.
fn format_size(bytes: i64) -> String {
    let bytes = bytes as f64;
    if bytes < 1024.0 {
        format!("{} B", bytes as i64)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    }
}
