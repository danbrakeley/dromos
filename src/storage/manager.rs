use rusqlite::Connection;
use std::fs;
use std::path::Path;

use crate::config::StorageConfig;
use crate::db::{NodeRow, Repository, run_migrations};
use crate::diff;
use crate::error::{DromosError, Result};
use crate::graph::{DiffEdge, PathStep, RomGraph, RomNode};
use crate::rom::{RomMetadata, format_hash, hash_rom_file, read_rom_bytes};

/// Result of removing a node
pub struct RemoveResult {
    pub title: String,
    pub edges_removed: usize,
    pub diff_files_removed: usize,
}

/// Result of building a ROM from diffs
pub struct BuildResult {
    pub bytes: Vec<u8>,
    pub target_row: NodeRow,
    pub steps: usize,
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

    /// Get full NodeRow from database (includes header metadata)
    pub fn get_node_row_by_hash(&self, sha256: &[u8; 32]) -> Result<Option<NodeRow>> {
        let repo = Repository::new(&self.conn);
        repo.get_node_by_hash(sha256)
    }

    /// Find path between two nodes by their hashes
    pub fn find_path(
        &self,
        source_hash: &[u8; 32],
        target_hash: &[u8; 32],
    ) -> Option<Vec<PathStep>> {
        let source_idx = self.graph.get_node_by_hash(source_hash)?;
        let target_idx = self.graph.get_node_by_hash(target_hash)?;
        self.graph.find_path(source_idx, target_idx)
    }

    /// Build a ROM by applying diffs from source to target
    pub fn build_rom(&self, source_path: &Path, target_hash: &[u8; 32]) -> Result<BuildResult> {
        // Get source metadata and verify it's in DB
        let source_meta = hash_rom_file(source_path)?;
        if self.get_node_by_hash(&source_meta.sha256).is_none() {
            return Err(DromosError::RomNotFound {
                hash: format_hash(&source_meta.sha256),
            });
        }

        // Find path
        let path = self
            .find_path(&source_meta.sha256, target_hash)
            .ok_or_else(|| DromosError::NoPath {
                from: format_hash(&source_meta.sha256),
                to: format_hash(target_hash),
            })?;

        // Read source bytes (headerless ROM data)
        let mut current_bytes = read_rom_bytes(source_path)?;

        // Apply each diff in the path
        for step in path.iter().skip(1) {
            // Skip source node
            if let Some(ref edge) = step.edge {
                let diff_path = self.config.diffs_dir.join(&edge.diff_path);
                current_bytes = diff::apply_diff(&current_bytes, &diff_path)?;
            }
        }

        // Get target node row (with header metadata)
        let target_row =
            self.get_node_row_by_hash(target_hash)?
                .ok_or_else(|| DromosError::RomNotFound {
                    hash: format_hash(target_hash),
                })?;

        Ok(BuildResult {
            bytes: current_bytes,
            target_row,
            steps: path.len() - 1,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::{Mirroring, NesHeader, RomMetadata, RomType};
    use rusqlite::Connection;
    use std::path::PathBuf;

    impl StorageManager {
        /// Create a StorageManager with in-memory database for testing
        pub fn new_in_memory(temp_dir: &Path) -> Result<Self> {
            let config = StorageConfig {
                db_path: PathBuf::from(":memory:"),
                diffs_dir: temp_dir.join("diffs"),
            };
            config.ensure_dirs_exist()?;

            let mut conn = Connection::open_in_memory()?;
            run_migrations(&mut conn)?;

            Ok(StorageManager {
                conn,
                graph: RomGraph::new(),
                config,
            })
        }

        /// Add a node directly from metadata (bypassing file I/O) for testing
        pub fn add_node_from_metadata(
            &mut self,
            metadata: &RomMetadata,
            title: &str,
        ) -> Result<()> {
            let repo = Repository::new(&self.conn);
            let db_id = repo.insert_node(metadata, title)?;

            self.graph.add_node(RomNode {
                db_id,
                sha256: metadata.sha256,
                filename: metadata.filename.clone(),
                title: title.to_string(),
                rom_type: metadata.rom_type,
            });

            Ok(())
        }
    }

    fn make_metadata(hash_byte: u8, filename: &str) -> RomMetadata {
        let mut sha256 = [0u8; 32];
        sha256[0] = hash_byte;
        RomMetadata {
            rom_type: RomType::Nes,
            sha256,
            filename: Some(filename.to_string()),
            nes_header: Some(NesHeader {
                prg_rom_size: 32 * 1024,
                chr_rom_size: 8 * 1024,
                has_trainer: false,
                mapper: 4,
                mirroring: Mirroring::Vertical,
                has_battery: true,
                is_nes2: false,
                submapper: None,
            }),
        }
    }

    #[test]
    fn test_add_node_and_retrieve() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let metadata = make_metadata(0xAA, "test.nes");
        manager
            .add_node_from_metadata(&metadata, "Test ROM")
            .unwrap();

        let node = manager
            .get_node_by_hash(&metadata.sha256)
            .expect("Node should exist");
        assert_eq!(node.title, "Test ROM");
        assert_eq!(node.sha256[0], 0xAA);
    }

    #[test]
    fn test_node_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let metadata = make_metadata(0xAA, "test.nes");

        assert!(!manager.node_exists(&metadata.sha256));

        manager
            .add_node_from_metadata(&metadata, "Test ROM")
            .unwrap();

        assert!(manager.node_exists(&metadata.sha256));
    }

    #[test]
    fn test_find_node_by_hash_prefix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let metadata = make_metadata(0xAB, "test.nes");
        manager
            .add_node_from_metadata(&metadata, "Test ROM")
            .unwrap();

        // Find by prefix "ab" (first byte is 0xAB)
        let node = manager
            .find_node_by_hash_prefix("ab")
            .expect("Should find by prefix");
        assert_eq!(node.title, "Test ROM");

        // Should not find with wrong prefix
        assert!(manager.find_node_by_hash_prefix("cd").is_none());
    }

    #[test]
    fn test_link_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");

        manager.add_node_from_metadata(&meta_a, "ROM A").unwrap();
        manager.add_node_from_metadata(&meta_b, "ROM B").unwrap();

        // Initially no links
        assert_eq!(manager.link_count(&meta_a.sha256), 0);

        // Manually add edge to the graph (bypassing file creation)
        let idx_a = manager.graph.get_node_by_hash(&meta_a.sha256).unwrap();
        let idx_b = manager.graph.get_node_by_hash(&meta_b.sha256).unwrap();
        manager.graph.add_edge(
            idx_a,
            idx_b,
            DiffEdge {
                db_id: 1,
                diff_path: "a_to_b.bsdiff".to_string(),
                diff_size: 100,
            },
        );

        assert_eq!(manager.link_count(&meta_a.sha256), 1);
        assert_eq!(manager.link_count(&meta_b.sha256), 0); // outgoing only
    }

    #[test]
    fn test_get_neighbors() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");
        let meta_c = make_metadata(0xCC, "c.nes");

        manager.add_node_from_metadata(&meta_a, "ROM A").unwrap();
        manager.add_node_from_metadata(&meta_b, "ROM B").unwrap();
        manager.add_node_from_metadata(&meta_c, "ROM C").unwrap();

        let idx_a = manager.graph.get_node_by_hash(&meta_a.sha256).unwrap();
        let idx_b = manager.graph.get_node_by_hash(&meta_b.sha256).unwrap();
        let idx_c = manager.graph.get_node_by_hash(&meta_c.sha256).unwrap();

        manager.graph.add_edge(
            idx_a,
            idx_b,
            DiffEdge {
                db_id: 1,
                diff_path: "a_to_b.bsdiff".to_string(),
                diff_size: 100,
            },
        );
        manager.graph.add_edge(
            idx_a,
            idx_c,
            DiffEdge {
                db_id: 2,
                diff_path: "a_to_c.bsdiff".to_string(),
                diff_size: 200,
            },
        );

        let neighbors = manager
            .get_neighbors(&meta_a.sha256)
            .expect("Should have neighbors");
        assert_eq!(neighbors.len(), 2);

        let titles: Vec<&str> = neighbors.iter().map(|(n, _)| n.title.as_str()).collect();
        assert!(titles.contains(&"ROM B"));
        assert!(titles.contains(&"ROM C"));
    }

    #[test]
    fn test_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");

        manager.add_node_from_metadata(&meta_a, "ROM A").unwrap();
        manager.add_node_from_metadata(&meta_b, "ROM B").unwrap();

        let (nodes, edges) = manager.list();
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges.len(), 0);
    }

    #[test]
    fn test_find_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = StorageManager::new_in_memory(temp_dir.path()).unwrap();

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");
        let meta_c = make_metadata(0xCC, "c.nes");

        manager.add_node_from_metadata(&meta_a, "ROM A").unwrap();
        manager.add_node_from_metadata(&meta_b, "ROM B").unwrap();
        manager.add_node_from_metadata(&meta_c, "ROM C").unwrap();

        let idx_a = manager.graph.get_node_by_hash(&meta_a.sha256).unwrap();
        let idx_b = manager.graph.get_node_by_hash(&meta_b.sha256).unwrap();
        let idx_c = manager.graph.get_node_by_hash(&meta_c.sha256).unwrap();

        manager.graph.add_edge(
            idx_a,
            idx_b,
            DiffEdge {
                db_id: 1,
                diff_path: "a_to_b.bsdiff".to_string(),
                diff_size: 100,
            },
        );
        manager.graph.add_edge(
            idx_b,
            idx_c,
            DiffEdge {
                db_id: 2,
                diff_path: "b_to_c.bsdiff".to_string(),
                diff_size: 100,
            },
        );

        let path = manager
            .find_path(&meta_a.sha256, &meta_c.sha256)
            .expect("Path should exist");
        assert_eq!(path.len(), 3); // A -> B -> C
    }
}
