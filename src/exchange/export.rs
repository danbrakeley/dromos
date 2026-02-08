use std::collections::HashSet;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::db::{DATA_REVISION, Repository};
use crate::error::{DromosError, Result};
use crate::graph::RomGraph;
use crate::rom::format_hash;

use super::format::{ExportEdge, ExportHeader, ExportManifest, ExportNode};

pub struct ExportStats {
    pub nodes: usize,
    pub edges: usize,
    pub aborted: bool,
}

pub enum OverwriteAction {
    Overwrite,
    Skip,
    Abort,
}

enum WriteResult {
    Written,
    Skipped,
    Aborted,
}

/// Write bytes to a file, calling `on_conflict` if the file already exists.
fn write_with_conflict_check(
    path: &Path,
    bytes: &[u8],
    on_conflict: &mut impl FnMut(&Path) -> Result<OverwriteAction>,
) -> Result<WriteResult> {
    if path.exists() {
        match on_conflict(path)? {
            OverwriteAction::Overwrite => {
                std::fs::write(path, bytes)?;
                Ok(WriteResult::Written)
            }
            OverwriteAction::Skip => Ok(WriteResult::Skipped),
            OverwriteAction::Abort => Ok(WriteResult::Aborted),
        }
    } else {
        std::fs::write(path, bytes)?;
        Ok(WriteResult::Written)
    }
}

/// Export nodes/edges to a folder.
///
/// If `component_hash` is provided, exports only the connected component
/// containing that node. Otherwise exports all nodes.
///
/// The `on_conflict` callback is called when a destination file already exists,
/// letting the caller decide whether to overwrite, skip, or abort.
pub fn write_folder(
    output_path: &Path,
    repo: &Repository,
    graph: &RomGraph,
    diffs_dir: &Path,
    component_hash: Option<&[u8; 32]>,
    on_conflict: &mut impl FnMut(&Path) -> Result<OverwriteAction>,
) -> Result<ExportStats> {
    // Determine which nodes to export
    let node_hashes: HashSet<[u8; 32]> = match component_hash {
        Some(hash) => {
            let start_idx = graph
                .get_node_by_hash(hash)
                .ok_or_else(|| DromosError::Export("Starting node not found in graph".into()))?;
            let component = graph.connected_component(start_idx);
            component
                .into_iter()
                .filter_map(|idx| graph.get_node(idx).map(|n| n.sha256))
                .collect()
        }
        None => graph.iter_nodes().map(|(_, n)| n.sha256).collect(),
    };

    // Load full NodeRows from DB for selected nodes
    let all_nodes = repo.load_all_nodes()?;
    let selected_nodes: Vec<_> = all_nodes
        .iter()
        .filter(|n| node_hashes.contains(&n.sha256))
        .collect();

    // Build a set of selected DB IDs for edge filtering
    let selected_ids: HashSet<i64> = selected_nodes.iter().map(|n| n.id).collect();

    // Build a DB ID -> hash string map for edge conversion
    let id_to_hash: std::collections::HashMap<i64, String> = selected_nodes
        .iter()
        .map(|n| (n.id, format_hash(&n.sha256)))
        .collect();

    // Load and filter edges to those within the selected set
    let all_edges = repo.load_all_edges()?;
    let selected_edges: Vec<_> = all_edges
        .iter()
        .filter(|e| selected_ids.contains(&e.source_id) && selected_ids.contains(&e.target_id))
        .collect();

    // Build manifest nodes
    let export_nodes: Vec<ExportNode> = selected_nodes
        .iter()
        .map(|n| ExportNode::from_node_row(n))
        .collect();

    // Read source diffs and compute SHA-256 hashes (without writing yet)
    let mut export_edges: Vec<ExportEdge> = Vec::new();
    let mut diff_data: Vec<(String, Vec<u8>)> = Vec::new();
    for e in &selected_edges {
        let diff_file_path = diffs_dir.join(&e.diff_path);
        let diff_sha256 = if diff_file_path.exists() {
            let diff_bytes = std::fs::read(&diff_file_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&diff_bytes);
            let hash_hex = hex::encode(hasher.finalize());
            diff_data.push((e.diff_path.clone(), diff_bytes));
            hash_hex
        } else {
            String::new()
        };

        export_edges.push(ExportEdge::from_edge_row(
            e,
            id_to_hash.get(&e.source_id).unwrap(),
            id_to_hash.get(&e.target_id).unwrap(),
            &diff_sha256,
        ));
    }

    let manifest = ExportManifest {
        dromos_export: ExportHeader {
            version: 1,
            data_revision: DATA_REVISION,
            exported_at: chrono::Utc::now().to_rfc3339(),
        },
        files: export_nodes,
        diffs: export_edges,
    };

    let node_count = manifest.files.len();
    let edge_count = manifest.diffs.len();
    let json = serde_json::to_string_pretty(&manifest)?;

    // Create output directory structure
    std::fs::create_dir_all(output_path).map_err(|e| {
        DromosError::Export(format!(
            "Failed to create directory {}: {}",
            output_path.display(),
            e
        ))
    })?;
    let output_diffs_dir = output_path.join("diffs");
    std::fs::create_dir_all(&output_diffs_dir)
        .map_err(|e| DromosError::Export(format!("Failed to create diffs directory: {}", e)))?;

    // Write index.json
    let index_path = output_path.join("index.json");
    if matches!(
        write_with_conflict_check(&index_path, json.as_bytes(), on_conflict)?,
        WriteResult::Aborted
    ) {
        return Ok(ExportStats {
            nodes: node_count,
            edges: edge_count,
            aborted: true,
        });
    }

    // Copy diff files
    for (filename, bytes) in &diff_data {
        let dest = output_diffs_dir.join(filename);
        if matches!(
            write_with_conflict_check(&dest, bytes, on_conflict)?,
            WriteResult::Aborted
        ) {
            return Ok(ExportStats {
                nodes: node_count,
                edges: edge_count,
                aborted: true,
            });
        }
    }

    Ok(ExportStats {
        nodes: node_count,
        edges: edge_count,
        aborted: false,
    })
}
