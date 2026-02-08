use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

use crate::db::{NodeRow, repository::EdgeRow};
use crate::rom::format_hash;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportManifest {
    pub dromos_export: ExportHeader,
    pub files: Vec<ExportNode>,
    pub diffs: Vec<ExportEdge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportHeader {
    pub version: u32,
    pub data_revision: u32,
    pub exported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportNode {
    pub sha256: String,
    pub filename: Option<String>,
    pub title: String,
    pub rom_type: String,
    pub version: Option<String>,
    pub source_url: Option<String>,
    pub release_date: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub source_file_header: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEdge {
    pub source_sha256: String,
    pub target_sha256: String,
    pub diff_path: String,
    pub diff_size: i64,
    pub sha256: String,
}

impl ExportNode {
    pub fn from_node_row(row: &NodeRow) -> Self {
        ExportNode {
            sha256: format_hash(&row.sha256),
            filename: row.filename.clone(),
            title: row.title.clone(),
            rom_type: row.rom_type.as_str().to_string(),
            version: row.version.clone(),
            source_url: row.source_url.clone(),
            release_date: row.release_date.clone(),
            tags: row.tags.clone(),
            description: row.description.clone(),
            source_file_header: row.source_file_header.as_ref().map(|h| BASE64.encode(h)),
        }
    }
}

impl ExportEdge {
    /// Create from an EdgeRow, resolving DB IDs to hash strings.
    pub fn from_edge_row(
        edge: &EdgeRow,
        source_hash: &str,
        target_hash: &str,
        diff_sha256: &str,
    ) -> Self {
        ExportEdge {
            source_sha256: source_hash.to_string(),
            target_sha256: target_hash.to_string(),
            diff_path: edge.diff_path.clone(),
            diff_size: edge.diff_size,
            sha256: diff_sha256.to_string(),
        }
    }
}
