use rusqlite::Connection;
use std::fs;
use std::path::Path;

use crate::config::StorageConfig;
use crate::db::{Repository, run_migrations};
use crate::diff;
use crate::error::Result;
use crate::graph::{DiffEdge, RomGraph, RomNode};
use crate::rom::{RomMetadata, format_hash, hash_rom_file, read_rom_bytes};

/// Result of removing a node
pub struct RemoveResult {
    pub title: String,
    pub edges_removed: usize,
    pub diff_files_removed: usize,
}

pub struct StorageManager {
    conn: Connection,
    graph: RomGraph,
    config: StorageConfig,
}

impl StorageManager {
    pub fn open(config: StorageConfig) -> Result<Self> {
        config.ensure_dirs_exist()?;

        let mut conn = Connection::open(&config.db_path)?;
        run_migrations(&mut conn)?;

        let mut manager = StorageManager {
            conn,
            graph: RomGraph::new(),
            config,
        };

        manager.load_graph_from_db()?;

        Ok(manager)
    }

    fn load_graph_from_db(&mut self) -> Result<()> {
        let repo = Repository::new(&self.conn);

        // Load all nodes
        let nodes = repo.load_all_nodes()?;
        for node_row in nodes {
            self.graph.add_node(RomNode {
                db_id: node_row.id,
                sha256: node_row.sha256,
                filename: node_row.filename,
                title: node_row.title,
                rom_type: node_row.rom_type,
            });
        }

        // Load all edges
        let edges = repo.load_all_edges()?;
        for edge_row in edges {
            if let (Some(source_idx), Some(target_idx)) = (
                self.graph.get_node_by_db_id(edge_row.source_id),
                self.graph.get_node_by_db_id(edge_row.target_id),
            ) {
                self.graph.add_edge(
                    source_idx,
                    target_idx,
                    DiffEdge {
                        db_id: edge_row.id,
                        diff_path: edge_row.diff_path,
                        diff_size: edge_row.diff_size,
                    },
                );
            }
        }

        Ok(())
    }

    pub fn add_node(&mut self, path: &Path, title: &str) -> Result<RomMetadata> {
        let metadata = hash_rom_file(path)?;

        let repo = Repository::new(&self.conn);

        let db_id = repo.insert_node(&metadata, title)?;

        self.graph.add_node(RomNode {
            db_id,
            sha256: metadata.sha256,
            filename: metadata.filename.clone(),
            title: title.to_string(),
            rom_type: metadata.rom_type,
        });

        Ok(metadata)
    }

    /// Get a node by hash, if it exists
    pub fn get_node_by_hash(&self, sha256: &[u8; 32]) -> Option<&RomNode> {
        self.graph
            .get_node_by_hash(sha256)
            .and_then(|idx| self.graph.get_node(idx))
    }

    /// Check if a ROM with the given hash exists
    pub fn node_exists(&self, sha256: &[u8; 32]) -> bool {
        self.graph.get_node_by_hash(sha256).is_some()
    }

    /// Create bidirectional links between two ROMs using their file paths.
    /// Both ROMs must already exist in the database.
    pub fn link_nodes(&mut self, path_a: &Path, path_b: &Path) -> Result<(u64, u64)> {
        let bytes_a = read_rom_bytes(path_a)?;
        let bytes_b = read_rom_bytes(path_b)?;

        let metadata_a = hash_rom_file(path_a)?;
        let metadata_b = hash_rom_file(path_b)?;

        let repo = Repository::new(&self.conn);

        // Get both nodes from the database
        let node_a = repo
            .get_node_by_hash(&metadata_a.sha256)?
            .expect("Node A must exist in database");
        let node_b = repo
            .get_node_by_hash(&metadata_b.sha256)?
            .expect("Node B must exist in database");

        // Create A -> B diff
        let diff_filename_ab = format!(
            "{}_{}.bsdiff",
            &format_hash(&metadata_a.sha256)[..16],
            &format_hash(&metadata_b.sha256)[..16]
        );
        let diff_path_ab = self.config.diffs_dir.join(&diff_filename_ab);
        let diff_size_ab = diff::create_diff(&bytes_a, &bytes_b, &diff_path_ab)?;

        // Create B -> A diff
        let diff_filename_ba = format!(
            "{}_{}.bsdiff",
            &format_hash(&metadata_b.sha256)[..16],
            &format_hash(&metadata_a.sha256)[..16]
        );
        let diff_path_ba = self.config.diffs_dir.join(&diff_filename_ba);
        let diff_size_ba = diff::create_diff(&bytes_b, &bytes_a, &diff_path_ba)?;

        // Insert edges
        repo.insert_edge(node_a.id, node_b.id, &diff_filename_ab, diff_size_ab as i64)?;
        repo.insert_edge(node_b.id, node_a.id, &diff_filename_ba, diff_size_ba as i64)?;

        // Update in-memory graph
        if let (Some(idx_a), Some(idx_b)) = (
            self.graph.get_node_by_db_id(node_a.id),
            self.graph.get_node_by_db_id(node_b.id),
        ) {
            self.graph.add_edge(
                idx_a,
                idx_b,
                DiffEdge {
                    db_id: 0,
                    diff_path: diff_filename_ab,
                    diff_size: diff_size_ab as i64,
                },
            );
            self.graph.add_edge(
                idx_b,
                idx_a,
                DiffEdge {
                    db_id: 0,
                    diff_path: diff_filename_ba,
                    diff_size: diff_size_ba as i64,
                },
            );
        }

        Ok((diff_size_ab, diff_size_ba))
    }

    pub fn list(&self) -> (Vec<&RomNode>, Vec<(String, String, i64)>) {
        let nodes: Vec<&RomNode> = self.graph.iter_nodes().map(|(_, n)| n).collect();

        let edges: Vec<(String, String, i64)> = self
            .graph
            .iter_edges()
            .filter_map(|(src, tgt, edge)| {
                let src_node = self.graph.get_node(src)?;
                let tgt_node = self.graph.get_node(tgt)?;
                Some((
                    format_hash(&src_node.sha256),
                    format_hash(&tgt_node.sha256),
                    edge.diff_size,
                ))
            })
            .collect();

        (nodes, edges)
    }

    /// Count outgoing links for a node
    pub fn link_count(&self, sha256: &[u8; 32]) -> usize {
        self.graph
            .get_node_by_hash(sha256)
            .map(|idx| self.graph.outgoing_edge_count(idx))
            .unwrap_or(0)
    }

    /// Get neighbors of a node by hash
    pub fn get_neighbors(&self, sha256: &[u8; 32]) -> Option<Vec<(&RomNode, i64)>> {
        let idx = self.graph.get_node_by_hash(sha256)?;
        Some(
            self.graph
                .neighbors(idx)
                .into_iter()
                .map(|(node, edge)| (node, edge.diff_size))
                .collect(),
        )
    }

    /// Find a node by hash prefix (for user convenience)
    pub fn find_node_by_hash_prefix(&self, prefix: &str) -> Option<&RomNode> {
        let prefix_lower = prefix.to_lowercase();
        self.graph
            .iter_nodes()
            .map(|(_, node)| node)
            .find(|node| format_hash(&node.sha256).starts_with(&prefix_lower))
    }

    /// Remove a node and all its associated links (edges and diff files)
    pub fn remove_node(&mut self, sha256: &[u8; 32]) -> Result<RemoveResult> {
        let repo = Repository::new(&self.conn);

        // Get the node from database
        let node_row = repo
            .get_node_by_hash(sha256)?
            .expect("Node must exist in database");

        let title = node_row.title.clone();

        // Get all edges involving this node
        let edges = repo.get_edges_for_node(node_row.id)?;
        let edges_removed = edges.len();

        // Delete diff files from disk (tolerating missing files)
        let mut diff_files_removed = 0;
        for edge in &edges {
            let diff_path = self.config.diffs_dir.join(&edge.diff_path);
            match fs::remove_file(&diff_path) {
                Ok(()) => diff_files_removed += 1,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!("Warning: diff file not found: {}", diff_path.display());
                }
                Err(e) => {
                    eprintln!("Warning: failed to delete {}: {}", diff_path.display(), e);
                }
            }
        }

        // Delete edges and node from database
        repo.delete_node(node_row.id)?;

        // Remove node from in-memory graph
        if let Some(idx) = self.graph.get_node_by_hash(sha256) {
            self.graph.remove_node(idx);
        }

        Ok(RemoveResult {
            title,
            edges_removed,
            diff_files_removed,
        })
    }
}
