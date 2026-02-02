use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::{DromosError, Result};
use crate::rom::{Mirroring, NesHeader, RomMetadata, RomType, format_hash};

/// Metadata for a ROM node (user-editable fields)
#[derive(Debug, Clone, Default)]
pub struct NodeMetadata {
    pub title: String,
    pub source_url: Option<String>,
    pub version: Option<String>,
    pub release_date: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
}

/// Map a database row to NodeRow. Expects columns in order:
/// id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size,
/// has_trainer, mapper, mirroring, has_battery, is_nes2, nes2_submapper,
/// source_url, version, release_date, tags, description
fn map_row_to_node_row(row: &Row) -> rusqlite::Result<NodeRow> {
    let hash_str: String = row.get(1)?;
    let sha256 = hex::decode(&hash_str)
        .ok()
        .and_then(|b| b.try_into().ok())
        .unwrap_or([0u8; 32]);
    let title: Option<String> = row.get(3)?;
    let rom_type_str: String = row.get(4)?;
    let rom_type = rom_type_str.parse().unwrap_or(RomType::Nes);
    let filename: Option<String> = row.get(2)?;

    // Parse tags from JSON array
    let tags_json: Option<String> = row.get(16)?;
    let tags = tags_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(NodeRow {
        id: row.get(0)?,
        sha256,
        title: title.unwrap_or_else(|| filename.clone().unwrap_or_default()),
        filename,
        rom_type,
        prg_rom_size: row.get::<_, Option<i64>>(5)?.map(|s| s as usize),
        chr_rom_size: row.get::<_, Option<i64>>(6)?.map(|s| s as usize),
        has_trainer: row.get(7)?,
        mapper: row.get::<_, Option<i64>>(8)?.map(|m| m as u16),
        mirroring: row
            .get::<_, Option<i64>>(9)?
            .map(|m| Mirroring::from(m as u8)),
        has_battery: row.get(10)?,
        is_nes2: row.get(11)?,
        submapper: row.get::<_, Option<i64>>(12)?.map(|s| s as u8),
        source_url: row.get(13)?,
        version: row.get(14)?,
        release_date: row.get(15)?,
        tags,
        description: row.get(17)?,
    })
}

#[derive(Debug, Clone)]
pub struct NodeRow {
    pub id: i64,
    pub sha256: [u8; 32],
    pub filename: Option<String>,
    pub title: String,
    pub rom_type: RomType,
    pub prg_rom_size: Option<usize>,
    pub chr_rom_size: Option<usize>,
    pub has_trainer: Option<bool>,
    pub mapper: Option<u16>,
    pub mirroring: Option<Mirroring>,
    pub has_battery: Option<bool>,
    pub is_nes2: Option<bool>,
    pub submapper: Option<u8>,
    // User-editable metadata
    pub source_url: Option<String>,
    pub version: Option<String>,
    pub release_date: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
}

impl NodeRow {
    /// Convert stored metadata to an NesHeader for file reconstruction.
    /// Returns None if required NES header fields are missing.
    pub fn to_nes_header(&self) -> Option<NesHeader> {
        Some(NesHeader {
            prg_rom_size: self.prg_rom_size?,
            chr_rom_size: self.chr_rom_size?,
            has_trainer: self.has_trainer.unwrap_or(false),
            mapper: self.mapper.unwrap_or(0),
            mirroring: self.mirroring.unwrap_or(Mirroring::Horizontal),
            has_battery: self.has_battery.unwrap_or(false),
            is_nes2: self.is_nes2.unwrap_or(false),
            submapper: self.submapper,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EdgeRow {
    pub id: i64,
    pub source_id: i64,
    pub target_id: i64,
    pub diff_path: String,
    pub diff_size: i64,
}

pub struct Repository<'a> {
    conn: &'a Connection,
}

impl<'a> Repository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Repository { conn }
    }

    pub fn insert_node(&self, metadata: &RomMetadata, node_metadata: &NodeMetadata) -> Result<i64> {
        let hash_hex = format_hash(&metadata.sha256);

        // Check if already exists
        if self.get_node_by_hash(&metadata.sha256)?.is_some() {
            return Err(DromosError::RomAlreadyExists { hash: hash_hex });
        }

        let (
            prg_rom_size,
            chr_rom_size,
            has_trainer,
            mapper,
            mirroring,
            has_battery,
            is_nes2,
            submapper,
        ) = match &metadata.nes_header {
            Some(h) => (
                Some(h.prg_rom_size),
                Some(h.chr_rom_size),
                Some(h.has_trainer),
                Some(h.mapper),
                Some(h.mirroring as u8),
                Some(h.has_battery),
                Some(h.is_nes2),
                h.submapper,
            ),
            None => (None, None, None, None, None, None, None, None),
        };

        // Serialize tags to JSON
        let tags_json = if node_metadata.tags.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&node_metadata.tags).unwrap_or_default())
        };

        self.conn.execute(
            "INSERT INTO nodes (sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer, mapper, mirroring, has_battery, is_nes2, nes2_submapper, source_url, version, release_date, tags, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                hash_hex,
                metadata.filename.as_deref(),
                &node_metadata.title,
                metadata.rom_type.as_str(),
                prg_rom_size.map(|s| s as i64),
                chr_rom_size.map(|s| s as i64),
                has_trainer,
                mapper.map(|m| m as i64),
                mirroring.map(|m| m as i64),
                has_battery,
                is_nes2,
                submapper.map(|s| s as i64),
                &node_metadata.source_url,
                &node_metadata.version,
                &node_metadata.release_date,
                &tags_json,
                &node_metadata.description,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_edge(
        &self,
        source_id: i64,
        target_id: i64,
        diff_path: &str,
        diff_size: i64,
    ) -> Result<i64> {
        // Check if edge already exists
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM edges WHERE source_id = ?1 AND target_id = ?2)",
            params![source_id, target_id],
            |row| row.get(0),
        )?;

        if exists {
            return Err(DromosError::DiffAlreadyExists(
                source_id.to_string(),
                target_id.to_string(),
            ));
        }

        self.conn.execute(
            "INSERT INTO edges (source_id, target_id, diff_path, diff_size)
             VALUES (?1, ?2, ?3, ?4)",
            params![source_id, target_id, diff_path, diff_size],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_node_by_hash(&self, sha256: &[u8; 32]) -> Result<Option<NodeRow>> {
        let hash_hex = format_hash(sha256);

        let result = self
            .conn
            .query_row(
                "SELECT id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer, mapper, mirroring, has_battery, is_nes2, nes2_submapper, source_url, version, release_date, tags, description
                 FROM nodes WHERE sha256 = ?1",
                params![hash_hex],
                map_row_to_node_row,
            )
            .optional()?;

        Ok(result)
    }

    pub fn get_node_by_id(&self, id: i64) -> Result<Option<NodeRow>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer, mapper, mirroring, has_battery, is_nes2, nes2_submapper, source_url, version, release_date, tags, description
                 FROM nodes WHERE id = ?1",
                params![id],
                map_row_to_node_row,
            )
            .optional()?;

        Ok(result)
    }

    pub fn load_all_nodes(&self) -> Result<Vec<NodeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer, mapper, mirroring, has_battery, is_nes2, nes2_submapper, source_url, version, release_date, tags, description
             FROM nodes ORDER BY id",
        )?;

        let rows = stmt.query_map([], map_row_to_node_row)?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }
        Ok(nodes)
    }

    pub fn load_all_edges(&self) -> Result<Vec<EdgeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, diff_path, diff_size
             FROM edges ORDER BY id",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(EdgeRow {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                diff_path: row.get(3)?,
                diff_size: row.get(4)?,
            })
        })?;

        let mut edges = Vec::new();
        for row in rows {
            edges.push(row?);
        }
        Ok(edges)
    }

    /// Get all edges involving a node (as source or target)
    pub fn get_edges_for_node(&self, node_id: i64) -> Result<Vec<EdgeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, diff_path, diff_size
             FROM edges WHERE source_id = ?1 OR target_id = ?1",
        )?;

        let rows = stmt.query_map(params![node_id], |row| {
            Ok(EdgeRow {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                diff_path: row.get(3)?,
                diff_size: row.get(4)?,
            })
        })?;

        let mut edges = Vec::new();
        for row in rows {
            edges.push(row?);
        }
        Ok(edges)
    }

    /// Delete all edges where source_id or target_id matches, then delete the node
    pub fn delete_node(&self, node_id: i64) -> Result<()> {
        // Delete all edges involving this node
        self.conn.execute(
            "DELETE FROM edges WHERE source_id = ?1 OR target_id = ?1",
            params![node_id],
        )?;

        // Delete the node itself
        self.conn
            .execute("DELETE FROM nodes WHERE id = ?1", params![node_id])?;

        Ok(())
    }

    /// Update metadata fields for a node
    pub fn update_node_metadata(&self, node_id: i64, metadata: &NodeMetadata) -> Result<()> {
        // Serialize tags to JSON
        let tags_json = if metadata.tags.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&metadata.tags).unwrap_or_default())
        };

        self.conn.execute(
            "UPDATE nodes SET title = ?1, source_url = ?2, version = ?3, release_date = ?4, tags = ?5, description = ?6 WHERE id = ?7",
            params![
                &metadata.title,
                &metadata.source_url,
                &metadata.version,
                &metadata.release_date,
                &tags_json,
                &metadata.description,
                node_id,
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        conn
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

    fn make_node_metadata(title: &str) -> NodeMetadata {
        NodeMetadata {
            title: title.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_insert_node() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        let id = repo.insert_node(&metadata, &node_meta).unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_insert_duplicate_node() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        repo.insert_node(&metadata, &node_meta).unwrap();

        // Second insert should fail
        let node_meta2 = make_node_metadata("Test ROM 2");
        let result = repo.insert_node(&metadata, &node_meta2);
        assert!(result.is_err());
        match result.unwrap_err() {
            DromosError::RomAlreadyExists { .. } => {}
            e => panic!("Expected RomAlreadyExists, got {:?}", e),
        }
    }

    #[test]
    fn test_get_node_by_hash() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo
            .get_node_by_hash(&metadata.sha256)
            .unwrap()
            .expect("Node should exist");

        assert_eq!(node.title, "Test ROM");
        assert_eq!(node.sha256[0], 0xAA);
        assert_eq!(node.rom_type, RomType::Nes);
        assert_eq!(node.prg_rom_size, Some(32 * 1024));
        assert_eq!(node.mapper, Some(4));
    }

    #[test]
    fn test_get_node_by_hash_not_found() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let mut missing_hash = [0u8; 32];
        missing_hash[0] = 0xFF;

        let result = repo.get_node_by_hash(&missing_hash).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_node_by_id() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        let id = repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo.get_node_by_id(id).unwrap().expect("Node should exist");
        assert_eq!(node.id, id);
        assert_eq!(node.title, "Test ROM");
    }

    #[test]
    fn test_insert_edge() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");

        let id_a = repo
            .insert_node(&meta_a, &make_node_metadata("ROM A"))
            .unwrap();
        let id_b = repo
            .insert_node(&meta_b, &make_node_metadata("ROM B"))
            .unwrap();

        let edge_id = repo.insert_edge(id_a, id_b, "a_to_b.bsdiff", 1234).unwrap();
        assert!(edge_id > 0);
    }

    #[test]
    fn test_insert_duplicate_edge() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");

        let id_a = repo
            .insert_node(&meta_a, &make_node_metadata("ROM A"))
            .unwrap();
        let id_b = repo
            .insert_node(&meta_b, &make_node_metadata("ROM B"))
            .unwrap();

        repo.insert_edge(id_a, id_b, "a_to_b.bsdiff", 1234).unwrap();

        // Second insert should fail
        let result = repo.insert_edge(id_a, id_b, "a_to_b_v2.bsdiff", 5678);
        assert!(result.is_err());
        match result.unwrap_err() {
            DromosError::DiffAlreadyExists(_, _) => {}
            e => panic!("Expected DiffAlreadyExists, got {:?}", e),
        }
    }

    #[test]
    fn test_load_all_nodes() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");

        repo.insert_node(&meta_a, &make_node_metadata("ROM A"))
            .unwrap();
        repo.insert_node(&meta_b, &make_node_metadata("ROM B"))
            .unwrap();

        let nodes = repo.load_all_nodes().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].title, "ROM A");
        assert_eq!(nodes[1].title, "ROM B");
    }

    #[test]
    fn test_load_all_edges() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");

        let id_a = repo
            .insert_node(&meta_a, &make_node_metadata("ROM A"))
            .unwrap();
        let id_b = repo
            .insert_node(&meta_b, &make_node_metadata("ROM B"))
            .unwrap();

        repo.insert_edge(id_a, id_b, "a_to_b.bsdiff", 1000).unwrap();
        repo.insert_edge(id_b, id_a, "b_to_a.bsdiff", 2000).unwrap();

        let edges = repo.load_all_edges().unwrap();
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].diff_path, "a_to_b.bsdiff");
        assert_eq!(edges[1].diff_path, "b_to_a.bsdiff");
    }

    #[test]
    fn test_delete_node_cascades_edges() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");
        let meta_c = make_metadata(0xCC, "c.nes");

        let id_a = repo
            .insert_node(&meta_a, &make_node_metadata("ROM A"))
            .unwrap();
        let id_b = repo
            .insert_node(&meta_b, &make_node_metadata("ROM B"))
            .unwrap();
        let id_c = repo
            .insert_node(&meta_c, &make_node_metadata("ROM C"))
            .unwrap();

        repo.insert_edge(id_a, id_b, "a_to_b.bsdiff", 1000).unwrap();
        repo.insert_edge(id_b, id_a, "b_to_a.bsdiff", 1000).unwrap();
        repo.insert_edge(id_b, id_c, "b_to_c.bsdiff", 1000).unwrap();

        // Delete node B
        repo.delete_node(id_b).unwrap();

        // Node B should be gone
        assert!(repo.get_node_by_id(id_b).unwrap().is_none());

        // All edges involving B should be gone
        let edges = repo.load_all_edges().unwrap();
        assert!(edges.is_empty());

        // Nodes A and C should still exist
        assert!(repo.get_node_by_id(id_a).unwrap().is_some());
        assert!(repo.get_node_by_id(id_c).unwrap().is_some());
    }

    #[test]
    fn test_get_edges_for_node() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let meta_a = make_metadata(0xAA, "a.nes");
        let meta_b = make_metadata(0xBB, "b.nes");
        let meta_c = make_metadata(0xCC, "c.nes");

        let id_a = repo
            .insert_node(&meta_a, &make_node_metadata("ROM A"))
            .unwrap();
        let id_b = repo
            .insert_node(&meta_b, &make_node_metadata("ROM B"))
            .unwrap();
        let id_c = repo
            .insert_node(&meta_c, &make_node_metadata("ROM C"))
            .unwrap();

        repo.insert_edge(id_a, id_b, "a_to_b.bsdiff", 1000).unwrap();
        repo.insert_edge(id_b, id_a, "b_to_a.bsdiff", 1000).unwrap();
        repo.insert_edge(id_b, id_c, "b_to_c.bsdiff", 1000).unwrap();
        repo.insert_edge(id_c, id_b, "c_to_b.bsdiff", 1000).unwrap();

        // Get edges for node B (should include all 4)
        let edges_b = repo.get_edges_for_node(id_b).unwrap();
        assert_eq!(edges_b.len(), 4);

        // Get edges for node A (should include 2: a_to_b and b_to_a)
        let edges_a = repo.get_edges_for_node(id_a).unwrap();
        assert_eq!(edges_a.len(), 2);
    }

    #[test]
    fn test_node_row_to_nes_header() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo
            .get_node_by_hash(&metadata.sha256)
            .unwrap()
            .expect("Node should exist");

        let header = node.to_nes_header().expect("Should convert to header");
        assert_eq!(header.prg_rom_size, 32 * 1024);
        assert_eq!(header.chr_rom_size, 8 * 1024);
        assert_eq!(header.mapper, 4);
        assert_eq!(header.mirroring, Mirroring::Vertical);
        assert!(header.has_battery);
    }

    #[test]
    fn test_insert_node_with_metadata() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = NodeMetadata {
            title: "Test ROM".to_string(),
            source_url: Some("https://example.com/rom".to_string()),
            version: Some("1.0".to_string()),
            release_date: Some("2024-01-15".to_string()),
            tags: vec!["action".to_string(), "platformer".to_string()],
            description: Some("A test ROM description".to_string()),
        };
        repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo
            .get_node_by_hash(&metadata.sha256)
            .unwrap()
            .expect("Node should exist");

        assert_eq!(node.title, "Test ROM");
        assert_eq!(node.source_url, Some("https://example.com/rom".to_string()));
        assert_eq!(node.version, Some("1.0".to_string()));
        assert_eq!(node.release_date, Some("2024-01-15".to_string()));
        assert_eq!(node.tags, vec!["action", "platformer"]);
        assert_eq!(node.description, Some("A test ROM description".to_string()));
    }

    #[test]
    fn test_insert_node_with_empty_metadata() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo
            .get_node_by_hash(&metadata.sha256)
            .unwrap()
            .expect("Node should exist");

        assert_eq!(node.title, "Test ROM");
        assert!(node.source_url.is_none());
        assert!(node.version.is_none());
        assert!(node.release_date.is_none());
        assert!(node.tags.is_empty());
        assert!(node.description.is_none());
    }

    #[test]
    fn test_update_node_metadata() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = make_node_metadata("Test ROM");
        let id = repo.insert_node(&metadata, &node_meta).unwrap();

        // Update metadata
        let updated_meta = NodeMetadata {
            title: "Updated ROM".to_string(),
            source_url: Some("https://new-url.com".to_string()),
            version: Some("2.0".to_string()),
            release_date: Some("2024-06-01".to_string()),
            tags: vec!["rpg".to_string()],
            description: Some("Updated description".to_string()),
        };
        repo.update_node_metadata(id, &updated_meta).unwrap();

        let node = repo.get_node_by_id(id).unwrap().expect("Node should exist");
        assert_eq!(node.title, "Updated ROM");
        assert_eq!(node.source_url, Some("https://new-url.com".to_string()));
        assert_eq!(node.version, Some("2.0".to_string()));
        assert_eq!(node.tags, vec!["rpg"]);
    }

    #[test]
    fn test_tags_json_roundtrip() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = NodeMetadata {
            title: "Test ROM".to_string(),
            tags: vec!["tag1".to_string(), "tag 2".to_string(), "tag-3".to_string()],
            ..Default::default()
        };
        repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo
            .get_node_by_hash(&metadata.sha256)
            .unwrap()
            .expect("Node should exist");

        assert_eq!(node.tags, vec!["tag1", "tag 2", "tag-3"]);
    }

    #[test]
    fn test_tags_empty_array() {
        let conn = setup_test_db();
        let repo = Repository::new(&conn);

        let metadata = make_metadata(0xAA, "test.nes");
        let node_meta = NodeMetadata {
            title: "Test ROM".to_string(),
            tags: vec![],
            ..Default::default()
        };
        repo.insert_node(&metadata, &node_meta).unwrap();

        let node = repo
            .get_node_by_hash(&metadata.sha256)
            .unwrap()
            .expect("Node should exist");

        assert!(node.tags.is_empty());
    }
}
