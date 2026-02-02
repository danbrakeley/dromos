use std::io::{self, Write};
use std::path::Path;

use rustyline::Editor;
use rustyline::history::DefaultHistory;

use crate::config::StorageConfig;
use crate::db::NodeMetadata;
use crate::error::Result;
use crate::graph::RomNode;
use crate::rom::{RomType, format_hash, hash_rom_file, reconstruct_nes_file};
use crate::storage::StorageManager;

use super::Command;
use super::completer::DromosHelper;
use super::multiline::edit_multiline;

pub struct ReplState {
    pub storage: StorageManager,
    pub last_added: Option<LastAdded>,
}

#[derive(Clone)]
pub struct LastAdded {
    pub hash: [u8; 32],
    pub title: String,
    pub version: Option<String>,
}

/// Result of ensuring a ROM is in the database
struct AddResult {
    title: String,
    version: Option<String>,
    hash: [u8; 32],
    newly_added: bool,
}

impl ReplState {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let storage = StorageManager::open(config)?;
        Ok(ReplState {
            storage,
            last_added: None,
        })
    }

    pub fn execute(
        &mut self,
        cmd: Command,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<bool> {
        match cmd {
            Command::Quit => return Ok(false),
            Command::Help => self.print_help(),
            Command::Hash { file } => self.cmd_hash(&file)?,
            Command::Add { file } => self.cmd_add(&file, rl)?,
            Command::Build { source, target } => self.cmd_build(&source, &target, rl)?,
            Command::Edit { target } => self.cmd_edit(&target, rl)?,
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
        println!("  build <source> <hash>   Build a ROM by applying diffs from source to target");
        println!("  edit <hash>             Edit metadata for a ROM");
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
            println!("Trainer: {}", if header.has_trainer { "Yes" } else { "No" });
        }

        Ok(())
    }

    /// Ensure a ROM file is in the database, prompting for metadata if new.
    /// Returns None if file doesn't exist (error already printed).
    /// Returns AddResult with newly_added=false if ROM already exists.
    /// Returns AddResult with newly_added=true if ROM was added.
    fn ensure_rom_added(
        &mut self,
        file: &Path,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<Option<AddResult>> {
        // Check if file exists
        if !file.exists() {
            eprintln!("File not found: {}", file.display());
            return Ok(None);
        }

        // Hash the file
        let metadata = hash_rom_file(file)?;

        // Check if ROM already exists
        if self.storage.node_exists(&metadata.sha256) {
            let node = self.storage.get_node_by_hash(&metadata.sha256).unwrap();
            return Ok(Some(AddResult {
                title: node.title.clone(),
                version: node.version.clone(),
                hash: metadata.sha256,
                newly_added: false,
            }));
        }

        // ROM doesn't exist - prompt for metadata and add
        let filename = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        println!("Adding file {}", filename);

        let default_title = title_from_filename(file);
        let node_metadata = prompt_metadata(rl, &default_title, None)?;

        // Add to database
        let metadata = self.storage.add_node(file, &node_metadata)?;

        let display_title =
            format_display_title(&node_metadata.title, node_metadata.version.as_deref());
        println!(
            "Added: {} ({}...)",
            display_title,
            &format_hash(&metadata.sha256)[..16]
        );

        Ok(Some(AddResult {
            title: node_metadata.title,
            version: node_metadata.version,
            hash: metadata.sha256,
            newly_added: true,
        }))
    }

    fn cmd_add(
        &mut self,
        file: &Path,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<()> {
        let result = match self.ensure_rom_added(file, rl)? {
            Some(r) => r,
            None => return Ok(()), // File not found, error already printed
        };

        if !result.newly_added {
            let display_title = format_display_title(&result.title, result.version.as_deref());
            println!(
                "ROM already exists: {} ({}...)",
                display_title,
                &format_hash(&result.hash)[..16]
            );
            return Ok(());
        }

        // Update last added
        self.last_added = Some(LastAdded {
            hash: result.hash,
            title: result.title,
            version: result.version,
        });

        Ok(())
    }

    fn cmd_build(
        &self,
        source: &Path,
        target: &str,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<()> {
        // Validate source exists
        if !source.exists() {
            eprintln!("File not found: {}", source.display());
            return Ok(());
        }

        // Find target node
        let target_node = match self.storage.find_node_by_hash_prefix(target) {
            Some(n) => n,
            None => {
                eprintln!("Target ROM not found: {}", target);
                return Ok(());
            }
        };
        let target_hash = target_node.sha256;
        let target_title = target_node.title.clone();
        let target_version = target_node.version.clone();
        let target_type = target_node.rom_type;

        // Build the ROM
        let display_title = format_display_title(&target_title, target_version.as_deref());
        println!("Building {}...", display_title);
        let result = match self.storage.build_rom(source, &target_hash) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Build failed: {}", e);
                return Ok(());
            }
        };
        println!("Applied {} diff(s)", result.steps);

        // Prompt for output filename
        let default_name = sanitize_filename(&target_title);
        let filename = prompt_with_initial(rl, "Output filename", &default_name)?;

        // Ensure correct extension
        let filename = ensure_extension(&filename, target_type);
        let output_path = Path::new(&filename);

        // Reconstruct with header for NES files
        let final_bytes = if target_type == RomType::Nes {
            if let Some(header) = result.target_row.to_nes_header() {
                reconstruct_nes_file(&header, &result.bytes)
            } else {
                eprintln!("Warning: No header metadata for NES file, writing raw bytes");
                result.bytes
            }
        } else {
            result.bytes
        };

        // Write to disk
        std::fs::write(output_path, &final_bytes)?;
        println!(
            "Wrote {} bytes to {}",
            final_bytes.len(),
            output_path.display()
        );

        Ok(())
    }

    fn cmd_link(
        &mut self,
        files: &[std::path::PathBuf],
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<()> {
        match files.len() {
            1 => self.link_to_last(&files[0], rl),
            2 => self.link_two_files(&files[0], &files[1], rl),
            _ => {
                eprintln!("Usage: link <file1> [file2]");
                Ok(())
            }
        }
    }

    fn link_to_last(
        &mut self,
        file: &Path,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<()> {
        let last = match &self.last_added {
            Some(last) => last.clone(),
            None => {
                eprintln!("No previous ROM to link to. Use 'link <file1> <file2>' instead.");
                return Ok(());
            }
        };

        // Confirm link to last added
        let last_display = format_display_title(&last.title, last.version.as_deref());
        let prompt = format!("Link to \"{}\"? [Y/n]", last_display);
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" || input == "no" {
            println!("Cancelled.");
            return Ok(());
        }

        // Add ROM if needed (with full metadata prompting)
        let result = match self.ensure_rom_added(file, rl)? {
            Some(r) => r,
            None => return Ok(()), // File not found, error already printed
        };

        // Need to find the file for the last_added ROM
        // For now, require the user to have the file accessible
        // This is a limitation - we'd need to store original file paths
        eprintln!(
            "Note: To create links, you need both ROM files. Use 'link <file1> <file2>' with both files."
        );

        // Update last added
        self.last_added = Some(LastAdded {
            hash: result.hash,
            title: result.title,
            version: result.version,
        });

        Ok(())
    }

    fn link_two_files(
        &mut self,
        file_a: &Path,
        file_b: &Path,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<()> {
        // Add first file if needed (with full metadata prompting)
        let result_a = match self.ensure_rom_added(file_a, rl)? {
            Some(r) => r,
            None => return Ok(()), // File not found, error already printed
        };

        // Add second file if needed (with full metadata prompting)
        let result_b = match self.ensure_rom_added(file_b, rl)? {
            Some(r) => r,
            None => return Ok(()), // File not found, error already printed
        };

        // Create bidirectional links
        self.storage.link_nodes(file_a, file_b)?;
        let display_a = format_display_title(&result_a.title, result_a.version.as_deref());
        let display_b = format_display_title(&result_b.title, result_b.version.as_deref());
        println!("Linked: {} <-> {}", display_a, display_b);

        // Update last added to the second file
        self.last_added = Some(LastAdded {
            hash: result_b.hash,
            title: result_b.title,
            version: result_b.version,
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
                format!(
                    "  [{} link{}]",
                    link_count,
                    if link_count == 1 { "" } else { "s" }
                )
            } else {
                String::new()
            };
            let display_title = format_display_title(&node.title, node.version.as_deref());
            println!(
                "{}  {}...  {}{}",
                display_title,
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

        let display_title = format_display_title(&node.title, node.version.as_deref());
        println!("{}  ({}...)", display_title, &format_hash(&node.sha256)[..16]);

        match neighbors {
            Some(links) if !links.is_empty() => {
                for (neighbor, diff_size) in links {
                    let neighbor_display =
                        format_display_title(&neighbor.title, neighbor.version.as_deref());
                    println!("  -> {}  ({})", neighbor_display, format_size(diff_size));
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
        let display_title = format_display_title(&node.title, node.version.as_deref());
        let link_count = self.storage.link_count(&sha256);

        // Prompt for confirmation
        let link_text = if link_count == 1 { "link" } else { "links" };
        print!(
            "Remove '{}' and {} {}? [y/N]: ",
            display_title, link_count, link_text
        );
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
            display_title,
            result.edges_removed,
            if result.edges_removed == 1 { "" } else { "s" },
            result.diff_files_removed,
            if result.diff_files_removed == 1 {
                ""
            } else {
                "s"
            }
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
            let display_title = format_display_title(&node.title, node.version.as_deref());
            println!(
                "{}  {}...  {}",
                display_title,
                &format_hash(&node.sha256)[..16],
                node.rom_type
            );
        }
    }

    fn cmd_edit(
        &mut self,
        target: &str,
        rl: &mut Editor<DromosHelper, DefaultHistory>,
    ) -> Result<()> {
        // Find node by hash prefix
        let node = match self.storage.find_node_by_hash_prefix(target) {
            Some(n) => n,
            None => {
                eprintln!("ROM not found: {}", target);
                return Ok(());
            }
        };

        let sha256 = node.sha256;

        // Get full NodeRow from database
        let node_row = match self.storage.get_node_row_by_hash(&sha256)? {
            Some(r) => r,
            None => {
                eprintln!("ROM not found in database: {}", target);
                return Ok(());
            }
        };

        // Prompt for updated metadata
        let node_metadata = prompt_metadata_from_row(rl, &node_row)?;

        // Update in storage
        self.storage.update_node_metadata(&sha256, &node_metadata)?;

        let display_title =
            format_display_title(&node_metadata.title, node_metadata.version.as_deref());
        println!(
            "Updated: {} ({}...)",
            display_title,
            &format_hash(&sha256)[..16]
        );

        Ok(())
    }
}

/// Format a title with optional version for display.
/// Returns "Title [version]" if version exists, otherwise just "Title".
fn format_display_title(title: &str, version: Option<&str>) -> String {
    match version {
        Some(v) if !v.is_empty() => format!("{} [{}]", title, v),
        _ => title.to_string(),
    }
}

/// Prompt the user with an editable initial value using rustyline.
fn prompt_with_initial(
    rl: &mut Editor<DromosHelper, DefaultHistory>,
    prompt: &str,
    initial: &str,
) -> Result<String> {
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

/// Prompt for an optional string field.
fn prompt_optional(
    rl: &mut Editor<DromosHelper, DefaultHistory>,
    prompt: &str,
    initial: Option<&str>,
) -> Result<Option<String>> {
    let initial_str = initial.unwrap_or("");
    let prompt_str = format!("{}: ", prompt);
    match rl.readline_with_initial(&prompt_str, (initial_str, "")) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Err(_) => Ok(initial.map(String::from)),
    }
}

/// Prompt for tags as comma-separated values.
fn prompt_tags(
    rl: &mut Editor<DromosHelper, DefaultHistory>,
    existing: &[String],
) -> Result<Vec<String>> {
    let initial = existing.join(", ");
    let prompt_str = "Tags (comma-separated): ";
    match rl.readline_with_initial(prompt_str, (&initial, "")) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                Ok(vec![])
            } else {
                Ok(trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect())
            }
        }
        Err(_) => Ok(existing.to_vec()),
    }
}

/// Prompt for a date in YYYY-MM-DD format.
fn prompt_date(
    rl: &mut Editor<DromosHelper, DefaultHistory>,
    existing: Option<&str>,
) -> Result<Option<String>> {
    let initial = existing.unwrap_or("");
    let prompt_str = "Release Date (YYYY-MM-DD): ";
    match rl.readline_with_initial(prompt_str, (initial, "")) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                // Validate date format
                if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
                    Ok(Some(date.format("%Y-%m-%d").to_string()))
                } else {
                    eprintln!("Invalid date format, expected YYYY-MM-DD");
                    Ok(existing.map(String::from))
                }
            }
        }
        Err(_) => Ok(existing.map(String::from)),
    }
}

/// Prompt for multi-line description.
fn prompt_description(existing: Option<&str>) -> Result<Option<String>> {
    let initial = existing.unwrap_or("");

    // Ask if user wants to enter/edit description
    print!("Description (press Enter to {}): ", if initial.is_empty() { "skip" } else { "edit" });
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().is_empty() && initial.is_empty() {
        return Ok(None);
    }

    if input.trim().is_empty() && !initial.is_empty() {
        // Keep existing description, don't open editor
        return Ok(Some(initial.to_string()));
    }

    // Open multi-line editor
    match edit_multiline("Description:", initial)? {
        Some(text) if text.trim().is_empty() => Ok(None),
        Some(text) => Ok(Some(text)),
        None => Ok(existing.map(String::from)),
    }
}

/// Prompt for all metadata fields when adding a new ROM.
fn prompt_metadata(
    rl: &mut Editor<DromosHelper, DefaultHistory>,
    default_title: &str,
    _existing: Option<&crate::db::NodeRow>,
) -> Result<NodeMetadata> {
    let title = prompt_with_initial(rl, "Title", default_title)?;
    let source_url = prompt_optional(rl, "Source URL", None)?;
    let version = prompt_optional(rl, "Version", None)?;
    let release_date = prompt_date(rl, None)?;
    let tags = prompt_tags(rl, &[])?;
    let description = prompt_description(None)?;

    Ok(NodeMetadata {
        title,
        source_url,
        version,
        release_date,
        tags,
        description,
    })
}

/// Prompt for all metadata fields when editing an existing ROM.
fn prompt_metadata_from_row(
    rl: &mut Editor<DromosHelper, DefaultHistory>,
    row: &crate::db::NodeRow,
) -> Result<NodeMetadata> {
    let title = prompt_with_initial(rl, "Title", &row.title)?;
    let source_url = prompt_optional(rl, "Source URL", row.source_url.as_deref())?;
    let version = prompt_optional(rl, "Version", row.version.as_deref())?;
    let release_date = prompt_date(rl, row.release_date.as_deref())?;
    let tags = prompt_tags(rl, &row.tags)?;
    let description = prompt_description(row.description.as_deref())?;

    Ok(NodeMetadata {
        title,
        source_url,
        version,
        release_date,
        tags,
        description,
    })
}

/// Known ROM file extensions to strip from titles.
const ROM_EXTENSIONS: &[&str] = &[
    ".nes", ".smc", ".sfc", ".gb", ".gbc", ".gba", ".nds", ".n64", ".z64", ".v64", ".gen", ".md",
    ".sms", ".gg", ".pce", ".bin", ".iso", ".cue", ".zip", ".7z",
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

/// Sanitize a string for use as a filename.
fn sanitize_filename(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Ensure filename has the correct extension for the ROM type.
fn ensure_extension(filename: &str, rom_type: RomType) -> String {
    let ext = match rom_type {
        RomType::Nes => ".nes",
    };
    if filename.to_lowercase().ends_with(ext) {
        filename.to_string()
    } else {
        format!("{}{}", filename, ext)
    }
}
