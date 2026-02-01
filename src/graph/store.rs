use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

use crate::rom::RomType;

#[derive(Debug, Clone)]
pub struct RomNode {
    pub db_id: i64,
    pub sha256: [u8; 32],
    pub filename: Option<String>,
    pub title: String,
    pub rom_type: RomType,
}

#[derive(Debug, Clone)]
pub struct DiffEdge {
    pub db_id: i64,
    pub diff_path: String,
    pub diff_size: i64,
}

pub struct RomGraph {
    graph: StableGraph<RomNode, DiffEdge>,
    hash_to_node: HashMap<[u8; 32], NodeIndex>,
    db_id_to_node: HashMap<i64, NodeIndex>,
}

impl RomGraph {
    pub fn new() -> Self {
        RomGraph {
            graph: StableGraph::new(),
            hash_to_node: HashMap::new(),
            db_id_to_node: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: RomNode) -> NodeIndex {
        let sha256 = node.sha256;
        let db_id = node.db_id;
        let idx = self.graph.add_node(node);
        self.hash_to_node.insert(sha256, idx);
        self.db_id_to_node.insert(db_id, idx);
        idx
    }

    pub fn add_edge(&mut self, source: NodeIndex, target: NodeIndex, edge: DiffEdge) {
        self.graph.add_edge(source, target, edge);
    }

    pub fn get_node_by_hash(&self, sha256: &[u8; 32]) -> Option<NodeIndex> {
        self.hash_to_node.get(sha256).copied()
    }

    pub fn get_node_by_db_id(&self, db_id: i64) -> Option<NodeIndex> {
        self.db_id_to_node.get(&db_id).copied()
    }

    pub fn get_node(&self, idx: NodeIndex) -> Option<&RomNode> {
        self.graph.node_weight(idx)
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (NodeIndex, &RomNode)> {
        self.graph.node_indices().filter_map(|idx| {
            self.graph.node_weight(idx).map(|node| (idx, node))
        })
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = (NodeIndex, NodeIndex, &DiffEdge)> {
        self.graph.edge_indices().filter_map(|idx| {
            let (source, target) = self.graph.edge_endpoints(idx)?;
            let edge = self.graph.edge_weight(idx)?;
            Some((source, target, edge))
        })
    }

    /// Count outgoing edges from a node
    pub fn outgoing_edge_count(&self, idx: NodeIndex) -> usize {
        self.graph.edges(idx).count()
    }

    /// Get all outgoing neighbors with their edge data
    pub fn neighbors(&self, idx: NodeIndex) -> Vec<(&RomNode, &DiffEdge)> {
        self.graph
            .edges(idx)
            .filter_map(|edge| {
                let target = edge.target();
                let node = self.graph.node_weight(target)?;
                let edge_data = edge.weight();
                Some((node, edge_data))
            })
            .collect()
    }
}

impl Default for RomGraph {
    fn default() -> Self {
        Self::new()
    }
}
