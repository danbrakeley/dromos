use rusqlite::Connection;
use std::path::Path;

use crate::cli::RootRef;
use crate::config::StorageConfig;
use crate::db::{run_migrations, Repository};
use crate::diff;
use crate::error::{DromosError, Result};
use crate::graph::{DiffEdge, RomGraph, RomNode};
use crate::rom::{format_hash, hash_rom_file, read_rom_bytes, RomMetadata};

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

    pub fn add_root(&mut self, path: &Path) -> Result<RomMetadata> {
        let metadata = hash_rom_file(path)?;

        let repo = Repository::new(&self.conn);

        let (prg_rom_size, chr_rom_size, has_trainer) = match &metadata.nes_header {
            Some(h) => (Some(h.prg_rom_size), Some(h.chr_rom_size), Some(h.has_trainer)),
            None => (None, None, None),
        };

        let db_id = repo.insert_node(
            &metadata.sha256,
            metadata.filename.as_deref(),
            metadata.rom_type,
            prg_rom_size,
            chr_rom_size,
            has_trainer,
        )?;

        self.graph.add_node(RomNode {
            db_id,
            sha256: metadata.sha256,
            filename: metadata.filename.clone(),
            rom_type: metadata.rom_type,
        });

        Ok(metadata)
    }

    pub fn add_mod(&mut self, root_ref: RootRef, mod_path: &Path) -> Result<RomMetadata> {
        // Find the root node
        let root_hash = match root_ref {
            RootRef::Hash(h) => h,
            RootRef::File(ref path) => {
                let metadata = hash_rom_file(path)?;
                metadata.sha256
            }
        };

        let repo = Repository::new(&self.conn);
        let root_node = repo
            .get_node_by_hash(&root_hash)?
            .ok_or_else(|| DromosError::RomNotFound {
                hash: format_hash(&root_hash),
            })?;

        // Hash the mod file
        let mod_metadata = hash_rom_file(mod_path)?;

        // Check if mod already exists
        if repo.get_node_by_hash(&mod_metadata.sha256)?.is_some() {
            return Err(DromosError::RomAlreadyExists {
                hash: format_hash(&mod_metadata.sha256),
            });
        }

        // Read both ROM contents for diffing
        // For the root, we need to find its original file or error
        let root_file_path = match &root_ref {
            RootRef::File(p) => p.clone(),
            RootRef::Hash(_) => {
                // If referenced by hash, we need the original file path from metadata
                // For now, require file reference for diff creation
                return Err(DromosError::FileNotFound {
                    path: std::path::PathBuf::from(format!(
                        "<hash:{}>",
                        format_hash(&root_hash)
                    )),
                });
            }
        };

        let old_bytes = read_rom_bytes(&root_file_path)?;
        let new_bytes = read_rom_bytes(mod_path)?;

        // Create diff file
        let diff_filename = format!(
            "{}_{}.bsdiff",
            &format_hash(&root_hash)[..16],
            &format_hash(&mod_metadata.sha256)[..16]
        );
        let diff_path = self.config.diffs_dir.join(&diff_filename);

        let diff_size = diff::create_diff(&old_bytes, &new_bytes, &diff_path)?;

        // Insert mod node
        let (prg_rom_size, chr_rom_size, has_trainer) = match &mod_metadata.nes_header {
            Some(h) => (Some(h.prg_rom_size), Some(h.chr_rom_size), Some(h.has_trainer)),
            None => (None, None, None),
        };

        let mod_db_id = repo.insert_node(
            &mod_metadata.sha256,
            mod_metadata.filename.as_deref(),
            mod_metadata.rom_type,
            prg_rom_size,
            chr_rom_size,
            has_trainer,
        )?;

        // Insert edge
        repo.insert_edge(
            root_node.id,
            mod_db_id,
            &diff_filename,
            diff_size as i64,
        )?;

        // Update graph
        let mod_node_idx = self.graph.add_node(RomNode {
            db_id: mod_db_id,
            sha256: mod_metadata.sha256,
            filename: mod_metadata.filename.clone(),
            rom_type: mod_metadata.rom_type,
        });

        if let Some(root_idx) = self.graph.get_node_by_db_id(root_node.id) {
            self.graph.add_edge(
                root_idx,
                mod_node_idx,
                DiffEdge {
                    db_id: 0, // Will be updated on next load
                    diff_path: diff_filename,
                    diff_size: diff_size as i64,
                },
            );
        }

        Ok(mod_metadata)
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

    pub fn resolve_root_ref(&self, root_ref: &RootRef) -> Result<[u8; 32]> {
        match root_ref {
            RootRef::Hash(h) => Ok(*h),
            RootRef::File(path) => {
                let metadata = hash_rom_file(path)?;
                Ok(metadata.sha256)
            }
        }
    }
}
