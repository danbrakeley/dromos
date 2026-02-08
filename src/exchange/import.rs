use std::collections::HashMap;
use std::fs;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use sha2::{Digest, Sha256};

use crate::db::{DATA_REVISION, NodeMetadata, Repository};
use crate::error::{DromosError, Result};
use crate::graph::{DiffEdge, RomGraph, RomNode};
use crate::rom::{RomMetadata, RomType, parse_hash};

use super::format::{ExportManifest, ExportNode};

/// Describes a field that differs between local and import data.
#[derive(Debug)]
pub struct FieldDiff {
    pub field: String,
    pub local_value: String,
    pub import_value: String,
}

/// A node that exists locally but has different metadata in the import.
#[derive(Debug)]
pub struct NodeConflict {
    pub sha256: String,
    pub title: String,
    pub diffs: Vec<FieldDiff>,
}

pub struct ImportResult {
    pub nodes_added: usize,
    pub nodes_skipped: usize,
    pub nodes_overwritten: usize,
    pub edges_added: usize,
    pub edges_skipped: usize,
    pub diffs_copied: usize,
}

/// Phase 1: Analyze a folder and identify conflicts.
pub fn analyze_import(
    folder_path: &Path,
    repo: &Repository,
) -> Result<(ExportManifest, Vec<NodeConflict>)> {
    // Read and parse index.json
    let index_path = folder_path.join("index.json");
    let json_str = fs::read_to_string(&index_path).map_err(|e| {
        DromosError::Import(format!("Failed to read {}: {}", index_path.display(), e))
    })?;
    let manifest: ExportManifest = serde_json::from_str(&json_str)?;

    // Validate data revision
    if manifest.dromos_export.data_revision != DATA_REVISION {
        return Err(DromosError::Import(format!(
            "Data revision mismatch: import has {}, local has {}",
            manifest.dromos_export.data_revision, DATA_REVISION
        )));
    }

    // Check each node for conflicts
    let mut conflicts = Vec::new();
    for import_node in &manifest.files {
        let hash = parse_hash(&import_node.sha256).ok_or_else(|| {
            DromosError::Import(format!("Invalid hash in import: {}", import_node.sha256))
        })?;

        if let Some(local_row) = repo.get_node_by_hash(&hash)? {
            let mut diffs = Vec::new();

            compare_field(&mut diffs, "title", &local_row.title, &import_node.title);
            compare_optional(
                &mut diffs,
                "version",
                &local_row.version,
                &import_node.version,
            );
            compare_optional(
                &mut diffs,
                "source_url",
                &local_row.source_url,
                &import_node.source_url,
            );
            compare_optional(
                &mut diffs,
                "release_date",
                &local_row.release_date,
                &import_node.release_date,
            );
            compare_optional(
                &mut diffs,
                "description",
                &local_row.description,
                &import_node.description,
            );

            let local_tags = local_row.tags.join(", ");
            let import_tags = import_node.tags.join(", ");
            if local_tags != import_tags {
                diffs.push(FieldDiff {
                    field: "tags".to_string(),
                    local_value: local_tags,
                    import_value: import_tags,
                });
            }

            if !diffs.is_empty() {
                conflicts.push(NodeConflict {
                    sha256: import_node.sha256.clone(),
                    title: import_node.title.clone(),
                    diffs,
                });
            }
        }
    }

    Ok((manifest, conflicts))
}

/// Phase 2: Execute the import, inserting nodes/edges and copying diffs.
pub fn execute_import(
    folder_path: &Path,
    manifest: &ExportManifest,
    overwrite: bool,
    repo: &Repository,
    graph: &mut RomGraph,
    diffs_dir: &Path,
) -> Result<ImportResult> {
    let mut result = ImportResult {
        nodes_added: 0,
        nodes_skipped: 0,
        nodes_overwritten: 0,
        edges_added: 0,
        edges_skipped: 0,
        diffs_copied: 0,
    };

    // Build hash -> DB ID map for edge insertion
    let mut hash_to_db_id: HashMap<String, i64> = HashMap::new();

    // Process nodes
    for import_node in &manifest.files {
        let hash = parse_hash(&import_node.sha256)
            .ok_or_else(|| DromosError::Import(format!("Invalid hash: {}", import_node.sha256)))?;

        if let Some(existing) = repo.get_node_by_hash(&hash)? {
            if overwrite {
                // Update metadata for conflicting nodes
                let node_meta = node_metadata_from_export(import_node);
                repo.update_node_metadata(existing.id, &node_meta)?;

                // Update in-memory graph
                if let Some(idx) = graph.get_node_by_hash(&hash)
                    && let Some(graph_node) = graph.get_node_mut(idx)
                {
                    graph_node.title = node_meta.title;
                    graph_node.version = node_meta.version;
                }

                result.nodes_overwritten += 1;
            } else {
                result.nodes_skipped += 1;
            }
            hash_to_db_id.insert(import_node.sha256.clone(), existing.id);
        } else {
            // New node: insert
            let rom_meta = rom_metadata_from_export(import_node)?;
            let node_meta = node_metadata_from_export(import_node);

            let db_id = repo.insert_node(&rom_meta, &node_meta)?;

            graph.add_node(RomNode {
                db_id,
                sha256: hash,
                filename: import_node.filename.clone(),
                title: node_meta.title.clone(),
                version: node_meta.version.clone(),
                rom_type: rom_meta.rom_type,
            });

            hash_to_db_id.insert(import_node.sha256.clone(), db_id);
            result.nodes_added += 1;
        }
    }

    // Process edges
    for import_edge in &manifest.diffs {
        let source_id = match hash_to_db_id.get(&import_edge.source_sha256) {
            Some(id) => *id,
            None => {
                // Source not in import set; try local DB
                let hash = parse_hash(&import_edge.source_sha256).ok_or_else(|| {
                    DromosError::Import(format!("Invalid hash: {}", import_edge.source_sha256))
                })?;
                match repo.get_node_by_hash(&hash)? {
                    Some(row) => row.id,
                    None => continue, // Skip edge if source not found
                }
            }
        };

        let target_id = match hash_to_db_id.get(&import_edge.target_sha256) {
            Some(id) => *id,
            None => {
                let hash = parse_hash(&import_edge.target_sha256).ok_or_else(|| {
                    DromosError::Import(format!("Invalid hash: {}", import_edge.target_sha256))
                })?;
                match repo.get_node_by_hash(&hash)? {
                    Some(row) => row.id,
                    None => continue,
                }
            }
        };

        // Try to insert edge; skip if it already exists
        match repo.insert_edge(
            source_id,
            target_id,
            &import_edge.diff_path,
            import_edge.diff_size,
        ) {
            Ok(edge_db_id) => {
                // Update in-memory graph
                let source_hash = parse_hash(&import_edge.source_sha256).unwrap();
                let target_hash = parse_hash(&import_edge.target_sha256).unwrap();

                if let (Some(src_idx), Some(tgt_idx)) = (
                    graph.get_node_by_hash(&source_hash),
                    graph.get_node_by_hash(&target_hash),
                ) {
                    graph.add_edge(
                        src_idx,
                        tgt_idx,
                        DiffEdge {
                            db_id: edge_db_id,
                            diff_path: import_edge.diff_path.clone(),
                            diff_size: import_edge.diff_size,
                        },
                    );
                }

                result.edges_added += 1;
            }
            Err(DromosError::DiffAlreadyExists(_, _)) => {
                result.edges_skipped += 1;
            }
            Err(e) => return Err(e),
        }
    }

    // Copy diff files from folder, verifying SHA-256
    let import_diffs_dir = folder_path.join("diffs");
    for import_edge in &manifest.diffs {
        let source_diff_path = import_diffs_dir.join(&import_edge.diff_path);
        let local_diff_path = diffs_dir.join(&import_edge.diff_path);

        // Skip if file already exists locally
        if local_diff_path.exists() {
            continue;
        }

        // Copy from import folder
        if source_diff_path.exists() {
            let bytes = fs::read(&source_diff_path)?;

            // Verify SHA-256 if checksum is present
            if !import_edge.sha256.is_empty() {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let computed = hex::encode(hasher.finalize());
                if computed != import_edge.sha256 {
                    return Err(DromosError::Import(format!(
                        "SHA-256 mismatch for {}: expected {}, got {}",
                        import_edge.diff_path, import_edge.sha256, computed
                    )));
                }
            }

            fs::write(&local_diff_path, &bytes)?;
            result.diffs_copied += 1;
        }
    }

    Ok(result)
}

fn compare_field(diffs: &mut Vec<FieldDiff>, field: &str, local: &str, import: &str) {
    if local != import {
        diffs.push(FieldDiff {
            field: field.to_string(),
            local_value: local.to_string(),
            import_value: import.to_string(),
        });
    }
}

fn compare_optional(
    diffs: &mut Vec<FieldDiff>,
    field: &str,
    local: &Option<String>,
    import: &Option<String>,
) {
    let local_str = local.as_deref().unwrap_or("");
    let import_str = import.as_deref().unwrap_or("");
    if local_str != import_str {
        diffs.push(FieldDiff {
            field: field.to_string(),
            local_value: local_str.to_string(),
            import_value: import_str.to_string(),
        });
    }
}

fn node_metadata_from_export(node: &ExportNode) -> NodeMetadata {
    NodeMetadata {
        title: node.title.clone(),
        source_url: node.source_url.clone(),
        version: node.version.clone(),
        release_date: node.release_date.clone(),
        tags: node.tags.clone(),
        description: node.description.clone(),
    }
}

fn rom_metadata_from_export(node: &ExportNode) -> Result<RomMetadata> {
    let sha256 = parse_hash(&node.sha256)
        .ok_or_else(|| DromosError::Import(format!("Invalid hash: {}", node.sha256)))?;

    let rom_type: RomType = node
        .rom_type
        .parse()
        .map_err(|_| DromosError::Import(format!("Unknown ROM type: {}", node.rom_type)))?;

    let source_file_header = node
        .source_file_header
        .as_ref()
        .and_then(|b64| BASE64.decode(b64).ok());

    Ok(RomMetadata {
        rom_type,
        sha256,
        filename: node.filename.clone(),
        nes_header: None, // Not serialized in export format
        source_file_header,
    })
}
