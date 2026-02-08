use petgraph::Direction;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::rom::RomType;

#[derive(Debug, Clone)]
pub struct RomNode {
    pub db_id: i64,
    pub sha256: [u8; 32],
    pub filename: Option<String>,
    pub title: String,
    pub version: Option<String>,
    pub rom_type: RomType,
}

#[derive(Debug, Clone)]
pub struct DiffEdge {
    pub db_id: i64,
    pub diff_path: String,
    pub diff_size: i64,
}

/// A step in a path from source to target node.
#[derive(Debug, Clone)]
pub struct PathStep {
    pub node_idx: NodeIndex,
    /// The edge used to reach this node. None for the source node.
    pub edge: Option<DiffEdge>,
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

    pub fn get_node_mut(&mut self, idx: NodeIndex) -> Option<&mut RomNode> {
        self.graph.node_weight_mut(idx)
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (NodeIndex, &RomNode)> {
        self.graph
            .node_indices()
            .filter_map(|idx| self.graph.node_weight(idx).map(|node| (idx, node)))
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

    /// Remove a node and all its edges from the graph, returning the removed node data
    pub fn remove_node(&mut self, idx: NodeIndex) -> Option<RomNode> {
        let node = self.graph.remove_node(idx)?;
        self.hash_to_node.remove(&node.sha256);
        self.db_id_to_node.remove(&node.db_id);
        Some(node)
    }

    /// Find shortest path from source to target using BFS.
    /// Returns None if no path exists.
    pub fn find_path(&self, source: NodeIndex, target: NodeIndex) -> Option<Vec<PathStep>> {
        if source == target {
            return Some(vec![PathStep {
                node_idx: source,
                edge: None,
            }]);
        }

        // visited maps each node to (previous node, edge used to reach it)
        let mut visited: HashMap<NodeIndex, (NodeIndex, DiffEdge)> = HashMap::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();
        queue.push_back(source);

        while let Some(current) = queue.pop_front() {
            for edge_ref in self.graph.edges(current) {
                let neighbor = edge_ref.target();
                if visited.contains_key(&neighbor) || neighbor == source {
                    continue;
                }
                visited.insert(neighbor, (current, edge_ref.weight().clone()));
                if neighbor == target {
                    return Some(self.reconstruct_path(source, target, &visited));
                }
                queue.push_back(neighbor);
            }
        }
        None
    }

    /// Find all nodes reachable from `start` treating edges as bidirectional.
    /// Uses BFS following both outgoing and incoming edges.
    pub fn connected_component(&self, start: NodeIndex) -> HashSet<NodeIndex> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            for edge_ref in self.graph.edges_directed(current, Direction::Outgoing) {
                let neighbor = edge_ref.target();
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
            for edge_ref in self.graph.edges_directed(current, Direction::Incoming) {
                let neighbor = edge_ref.source();
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        visited
    }

    fn reconstruct_path(
        &self,
        source: NodeIndex,
        target: NodeIndex,
        visited: &HashMap<NodeIndex, (NodeIndex, DiffEdge)>,
    ) -> Vec<PathStep> {
        let mut path = Vec::new();
        let mut current = target;

        while current != source {
            let (prev, edge) = visited.get(&current).unwrap();
            path.push(PathStep {
                node_idx: current,
                edge: Some(edge.clone()),
            });
            current = *prev;
        }
        path.push(PathStep {
            node_idx: source,
            edge: None,
        });
        path.reverse();
        path
    }
}

impl Default for RomGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(db_id: i64, hash_byte: u8, title: &str) -> RomNode {
        let mut sha256 = [0u8; 32];
        sha256[0] = hash_byte;
        RomNode {
            db_id,
            sha256,
            filename: Some(format!("{}.nes", title)),
            title: title.to_string(),
            version: None,
            rom_type: RomType::Nes,
        }
    }

    fn make_edge(db_id: i64, diff_path: &str) -> DiffEdge {
        DiffEdge {
            db_id,
            diff_path: diff_path.to_string(),
            diff_size: 100,
        }
    }

    #[test]
    fn test_add_node() {
        let mut graph = RomGraph::new();
        let node = make_node(1, 0xAA, "Test ROM");
        let idx = graph.add_node(node.clone());

        assert_eq!(graph.node_count(), 1);
        let retrieved = graph.get_node(idx).unwrap();
        assert_eq!(retrieved.db_id, 1);
        assert_eq!(retrieved.title, "Test ROM");
        assert_eq!(retrieved.sha256[0], 0xAA);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = RomGraph::new();
        let node_a = make_node(1, 0xAA, "ROM A");
        let node_b = make_node(2, 0xBB, "ROM B");

        let idx_a = graph.add_node(node_a);
        let idx_b = graph.add_node(node_b);

        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.outgoing_edge_count(idx_a), 1);
        assert_eq!(graph.outgoing_edge_count(idx_b), 0);
    }

    #[test]
    fn test_get_node_by_hash() {
        let mut graph = RomGraph::new();
        let node = make_node(1, 0xAA, "Test ROM");
        let sha256 = node.sha256;
        graph.add_node(node);

        let idx = graph.get_node_by_hash(&sha256);
        assert!(idx.is_some());

        let mut missing_hash = [0u8; 32];
        missing_hash[0] = 0xFF;
        assert!(graph.get_node_by_hash(&missing_hash).is_none());
    }

    #[test]
    fn test_get_node_by_db_id() {
        let mut graph = RomGraph::new();
        let node = make_node(42, 0xAA, "Test ROM");
        graph.add_node(node);

        let idx = graph.get_node_by_db_id(42);
        assert!(idx.is_some());

        assert!(graph.get_node_by_db_id(999).is_none());
    }

    #[test]
    fn test_find_path_direct() {
        let mut graph = RomGraph::new();
        let node_a = make_node(1, 0xAA, "ROM A");
        let node_b = make_node(2, 0xBB, "ROM B");

        let idx_a = graph.add_node(node_a);
        let idx_b = graph.add_node(node_b);
        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));

        let path = graph.find_path(idx_a, idx_b).expect("Path should exist");
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].node_idx, idx_a);
        assert!(path[0].edge.is_none());
        assert_eq!(path[1].node_idx, idx_b);
        assert!(path[1].edge.is_some());
        assert_eq!(path[1].edge.as_ref().unwrap().diff_path, "a_to_b.bsdiff");
    }

    #[test]
    fn test_find_path_multi_hop() {
        let mut graph = RomGraph::new();
        let node_a = make_node(1, 0xAA, "ROM A");
        let node_b = make_node(2, 0xBB, "ROM B");
        let node_c = make_node(3, 0xCC, "ROM C");

        let idx_a = graph.add_node(node_a);
        let idx_b = graph.add_node(node_b);
        let idx_c = graph.add_node(node_c);

        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));
        graph.add_edge(idx_b, idx_c, make_edge(2, "b_to_c.bsdiff"));

        let path = graph.find_path(idx_a, idx_c).expect("Path should exist");
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].node_idx, idx_a);
        assert_eq!(path[1].node_idx, idx_b);
        assert_eq!(path[2].node_idx, idx_c);
    }

    #[test]
    fn test_find_path_no_route() {
        let mut graph = RomGraph::new();
        let node_a = make_node(1, 0xAA, "ROM A");
        let node_b = make_node(2, 0xBB, "ROM B");

        let idx_a = graph.add_node(node_a);
        let idx_b = graph.add_node(node_b);
        // No edge between them

        let path = graph.find_path(idx_a, idx_b);
        assert!(path.is_none());
    }

    #[test]
    fn test_find_path_same_node() {
        let mut graph = RomGraph::new();
        let node = make_node(1, 0xAA, "ROM A");
        let idx = graph.add_node(node);

        let path = graph.find_path(idx, idx).expect("Path should exist");
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].node_idx, idx);
        assert!(path[0].edge.is_none());
    }

    #[test]
    fn test_remove_node() {
        let mut graph = RomGraph::new();
        let node = make_node(1, 0xAA, "ROM A");
        let sha256 = node.sha256;
        let idx = graph.add_node(node);

        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node_by_hash(&sha256).is_some());
        assert!(graph.get_node_by_db_id(1).is_some());

        let removed = graph.remove_node(idx);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().title, "ROM A");

        assert_eq!(graph.node_count(), 0);
        assert!(graph.get_node_by_hash(&sha256).is_none());
        assert!(graph.get_node_by_db_id(1).is_none());
    }

    #[test]
    fn test_connected_component_single_node() {
        let mut graph = RomGraph::new();
        let node = make_node(1, 0xAA, "ROM A");
        let idx = graph.add_node(node);

        let component = graph.connected_component(idx);
        assert_eq!(component.len(), 1);
        assert!(component.contains(&idx));
    }

    #[test]
    fn test_connected_component_chain() {
        let mut graph = RomGraph::new();
        let idx_a = graph.add_node(make_node(1, 0xAA, "ROM A"));
        let idx_b = graph.add_node(make_node(2, 0xBB, "ROM B"));
        let idx_c = graph.add_node(make_node(3, 0xCC, "ROM C"));

        // A -> B -> C (one-way edges)
        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));
        graph.add_edge(idx_b, idx_c, make_edge(2, "b_to_c.bsdiff"));

        // Starting from any node should find all three (bidirectional traversal)
        let component = graph.connected_component(idx_a);
        assert_eq!(component.len(), 3);
        assert!(component.contains(&idx_a));
        assert!(component.contains(&idx_b));
        assert!(component.contains(&idx_c));

        let component_c = graph.connected_component(idx_c);
        assert_eq!(component_c.len(), 3);
    }

    #[test]
    fn test_connected_component_two_separate() {
        let mut graph = RomGraph::new();
        let idx_a = graph.add_node(make_node(1, 0xAA, "ROM A"));
        let idx_b = graph.add_node(make_node(2, 0xBB, "ROM B"));
        let idx_c = graph.add_node(make_node(3, 0xCC, "ROM C"));
        let idx_d = graph.add_node(make_node(4, 0xDD, "ROM D"));

        // Component 1: A <-> B
        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));
        // Component 2: C <-> D
        graph.add_edge(idx_c, idx_d, make_edge(2, "c_to_d.bsdiff"));

        let comp_a = graph.connected_component(idx_a);
        assert_eq!(comp_a.len(), 2);
        assert!(comp_a.contains(&idx_a));
        assert!(comp_a.contains(&idx_b));
        assert!(!comp_a.contains(&idx_c));

        let comp_c = graph.connected_component(idx_c);
        assert_eq!(comp_c.len(), 2);
        assert!(comp_c.contains(&idx_c));
        assert!(comp_c.contains(&idx_d));
    }

    #[test]
    fn test_connected_component_undirected_traversal() {
        let mut graph = RomGraph::new();
        let idx_a = graph.add_node(make_node(1, 0xAA, "ROM A"));
        let idx_b = graph.add_node(make_node(2, 0xBB, "ROM B"));
        let idx_c = graph.add_node(make_node(3, 0xCC, "ROM C"));

        // Only one-way edges: A -> B, C -> B
        // B has no outgoing edges, but should still be reachable from A via incoming on C
        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));
        graph.add_edge(idx_c, idx_b, make_edge(2, "c_to_b.bsdiff"));

        // Starting from A: A -> B (outgoing), B <- C (incoming on B), so all connected
        let component = graph.connected_component(idx_a);
        assert_eq!(component.len(), 3);
    }

    #[test]
    fn test_neighbors() {
        let mut graph = RomGraph::new();
        let node_a = make_node(1, 0xAA, "ROM A");
        let node_b = make_node(2, 0xBB, "ROM B");
        let node_c = make_node(3, 0xCC, "ROM C");

        let idx_a = graph.add_node(node_a);
        let idx_b = graph.add_node(node_b);
        let idx_c = graph.add_node(node_c);

        graph.add_edge(idx_a, idx_b, make_edge(1, "a_to_b.bsdiff"));
        graph.add_edge(idx_a, idx_c, make_edge(2, "a_to_c.bsdiff"));

        let neighbors = graph.neighbors(idx_a);
        assert_eq!(neighbors.len(), 2);

        let titles: Vec<&str> = neighbors.iter().map(|(n, _)| n.title.as_str()).collect();
        assert!(titles.contains(&"ROM B"));
        assert!(titles.contains(&"ROM C"));
    }
}
