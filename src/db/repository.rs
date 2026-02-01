use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{DromosError, Result};
use crate::rom::{RomMetadata, RomType, format_hash};

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

    pub fn insert_node(&self, metadata: &RomMetadata, title: &str) -> Result<i64> {
        let hash_hex = format_hash(&metadata.sha256);

        // Check if already exists
        if self.get_node_by_hash(&metadata.sha256)?.is_some() {
            return Err(DromosError::RomAlreadyExists { hash: hash_hex });
        }

        let (prg_rom_size, chr_rom_size, has_trainer) = match &metadata.nes_header {
            Some(h) => (
                Some(h.prg_rom_size),
                Some(h.chr_rom_size),
                Some(h.has_trainer),
            ),
            None => (None, None, None),
        };

        self.conn.execute(
            "INSERT INTO nodes (sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                hash_hex,
                metadata.filename.as_deref(),
                title,
                metadata.rom_type.as_str(),
                prg_rom_size.map(|s| s as i64),
                chr_rom_size.map(|s| s as i64),
                has_trainer,
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
                "SELECT id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer
                 FROM nodes WHERE sha256 = ?1",
                params![hash_hex],
                |row| {
                    let hash_str: String = row.get(1)?;
                    let sha256 = hex::decode(&hash_str)
                        .ok()
                        .and_then(|b| b.try_into().ok())
                        .unwrap_or([0u8; 32]);
                    let title: Option<String> = row.get(3)?;
                    let rom_type_str: String = row.get(4)?;
                    let rom_type = rom_type_str.parse().unwrap_or(RomType::Nes);
                    let filename: Option<String> = row.get(2)?;

                    Ok(NodeRow {
                        id: row.get(0)?,
                        sha256,
                        title: title.unwrap_or_else(|| filename.clone().unwrap_or_default()),
                        filename,
                        rom_type,
                        prg_rom_size: row.get::<_, Option<i64>>(5)?.map(|s| s as usize),
                        chr_rom_size: row.get::<_, Option<i64>>(6)?.map(|s| s as usize),
                        has_trainer: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn get_node_by_id(&self, id: i64) -> Result<Option<NodeRow>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer
                 FROM nodes WHERE id = ?1",
                params![id],
                |row| {
                    let hash_str: String = row.get(1)?;
                    let sha256 = hex::decode(&hash_str)
                        .ok()
                        .and_then(|b| b.try_into().ok())
                        .unwrap_or([0u8; 32]);
                    let title: Option<String> = row.get(3)?;
                    let rom_type_str: String = row.get(4)?;
                    let rom_type = rom_type_str.parse().unwrap_or(RomType::Nes);
                    let filename: Option<String> = row.get(2)?;

                    Ok(NodeRow {
                        id: row.get(0)?,
                        sha256,
                        title: title.unwrap_or_else(|| filename.clone().unwrap_or_default()),
                        filename,
                        rom_type,
                        prg_rom_size: row.get::<_, Option<i64>>(5)?.map(|s| s as usize),
                        chr_rom_size: row.get::<_, Option<i64>>(6)?.map(|s| s as usize),
                        has_trainer: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn load_all_nodes(&self) -> Result<Vec<NodeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sha256, filename, title, rom_type, prg_rom_size, chr_rom_size, has_trainer
             FROM nodes ORDER BY id",
        )?;

        let rows = stmt.query_map([], |row| {
            let hash_str: String = row.get(1)?;
            let sha256 = hex::decode(&hash_str)
                .ok()
                .and_then(|b| b.try_into().ok())
                .unwrap_or([0u8; 32]);
            let title: Option<String> = row.get(3)?;
            let rom_type_str: String = row.get(4)?;
            let rom_type = rom_type_str.parse().unwrap_or(RomType::Nes);
            let filename: Option<String> = row.get(2)?;

            Ok(NodeRow {
                id: row.get(0)?,
                sha256,
                title: title.unwrap_or_else(|| filename.clone().unwrap_or_default()),
                filename,
                rom_type,
                prg_rom_size: row.get::<_, Option<i64>>(5)?.map(|s| s as usize),
                chr_rom_size: row.get::<_, Option<i64>>(6)?.map(|s| s as usize),
                has_trainer: row.get(7)?,
            })
        })?;

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
}
