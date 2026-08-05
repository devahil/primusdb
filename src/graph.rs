/*
 * PrimusDB - Graph Database Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.3.1-alpha - Added property graph model
 */

/*!
# PrimusDB Graph Database Engine

An in-memory property graph database engine supporting the property graph model
with nodes and directed edges, labels, and key-value properties.

## Property Graph Model

```text
Property Graph Model
================================================

+------------------+          +------------------+
|    GraphNode     |          |    GraphEdge     |
+------------------+          +------------------+
| id: NodeId       |          | id: EdgeId       |
| labels: Vec      |--------->| source: NodeId   |
| properties: Map  |          | target: NodeId   |
+------------------+          | label: String    |
                              | properties: Map  |
                              +------------------+

Features:
- Nodes with multiple labels and arbitrary JSON properties
- Directed edges with labels and properties
- Automatic adjacency indexing (outgoing/incoming edges)
- BFS traversal with direction and label filtering
- Simple graph query execution
```

## Usage Example

```ignore
use primusdb::graph::{GraphEngine, TraversalDirection, GraphQuery};
use std::collections::HashMap;

let mut graph = GraphEngine::new();

// Create nodes
let alice = graph.add_node(
    vec!["Person".to_string()],
    HashMap::from([("name".to_string(), serde_json::json!("Alice"))]),
);
let bob = graph.add_node(
    vec!["Person".to_string()],
    HashMap::from([("name".to_string(), serde_json::json!("Bob"))]),
);

// Create an edge
graph.add_edge(alice, bob, "knows".to_string(), HashMap::new()).unwrap();

// Traverse
let friends = graph.traverse(alice, "knows", TraversalDirection::Outgoing);
assert_eq!(friends.len(), 1);
assert_eq!(friends[0].properties["name"], serde_json::json!("Bob"));
```
*/

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A unique identifier for a graph node
pub type NodeId = u64;

/// A unique identifier for a graph edge
pub type EdgeId = u64;

/// A graph node with labels and properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier for this node
    pub id: NodeId,
    /// Labels assigned to this node (e.g., "Person", "Company")
    pub labels: Vec<String>,
    /// Arbitrary key-value properties stored as JSON values
    pub properties: HashMap<String, serde_json::Value>,
}

/// A directed edge between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique identifier for this edge
    pub id: EdgeId,
    /// ID of the source node
    pub source: NodeId,
    /// ID of the target node
    pub target: NodeId,
    /// Label describing the relationship (e.g., "knows", "works_at")
    pub label: String,
    /// Arbitrary key-value properties stored as JSON values
    pub properties: HashMap<String, serde_json::Value>,
}

/// In-memory property graph engine
///
/// Maintains nodes, directed edges, and adjacency indexes for efficient
/// traversal queries. All operations are in-memory and serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEngine {
    nodes: HashMap<NodeId, GraphNode>,
    edges: HashMap<EdgeId, GraphEdge>,
    /// Outgoing edges index: node_id -> Vec<edge_id>
    out_edges: HashMap<NodeId, Vec<EdgeId>>,
    /// Incoming edges index: node_id -> Vec<edge_id>
    in_edges: HashMap<NodeId, Vec<EdgeId>>,
    next_node_id: NodeId,
    next_edge_id: EdgeId,
}

impl GraphEngine {
    /// Creates a new empty graph engine
    pub fn new() -> Self {
        GraphEngine {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            out_edges: HashMap::new(),
            in_edges: HashMap::new(),
            next_node_id: 0,
            next_edge_id: 0,
        }
    }

    /// Add a node with given labels and properties
    ///
    /// Returns the auto-generated unique node ID.
    pub fn add_node(
        &mut self,
        labels: Vec<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        self.nodes.insert(
            id,
            GraphNode {
                id,
                labels,
                properties,
            },
        );
        self.out_edges.entry(id).or_default();
        self.in_edges.entry(id).or_default();
        id
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.get(&id)
    }

    /// Remove a node and all its edges
    ///
    /// Returns `true` if the node existed and was removed.
    /// All edges incident to this node are also removed.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if !self.nodes.contains_key(&id) {
            return false;
        }

        // Collect all edge IDs to remove
        let mut edges_to_remove = Vec::new();
        if let Some(out) = self.out_edges.get(&id) {
            edges_to_remove.extend(out.iter());
        }
        if let Some(in_) = self.in_edges.get(&id) {
            edges_to_remove.extend(in_.iter());
        }

        for &eid in &edges_to_remove {
            self.edges.remove(&eid);
        }

        // Remove from adjacency indexes of neighbor nodes
        if let Some(out) = self.out_edges.remove(&id) {
            for &eid in &out {
                if let Some(edge) = self.edges.get(&eid) {
                    if let Some(in_edges) = self.in_edges.get_mut(&edge.target) {
                        in_edges.retain(|&e| e != eid);
                    }
                }
            }
        }
        if let Some(in_) = self.in_edges.remove(&id) {
            for &eid in &in_ {
                if let Some(edge) = self.edges.get(&eid) {
                    if let Some(out_edges) = self.out_edges.get_mut(&edge.source) {
                        out_edges.retain(|&e| e != eid);
                    }
                }
            }
        }

        self.nodes.remove(&id);
        true
    }

    /// Add a directed edge between two nodes
    ///
    /// Returns an error if either source or target node does not exist.
    pub fn add_edge(
        &mut self,
        source: NodeId,
        target: NodeId,
        label: String,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<EdgeId, String> {
        if !self.nodes.contains_key(&source) {
            return Err(format!("Source node {} does not exist", source));
        }
        if !self.nodes.contains_key(&target) {
            return Err(format!("Target node {} does not exist", target));
        }
        let id = self.next_edge_id;
        self.next_edge_id += 1;
        self.edges.insert(
            id,
            GraphEdge {
                id,
                source,
                target,
                label,
                properties,
            },
        );
        self.out_edges.entry(source).or_default().push(id);
        self.in_edges.entry(target).or_default().push(id);
        Ok(id)
    }

    /// Get an edge by ID
    pub fn get_edge(&self, id: EdgeId) -> Option<&GraphEdge> {
        self.edges.get(&id)
    }

    /// Remove an edge
    ///
    /// Returns `true` if the edge existed and was removed.
    pub fn remove_edge(&mut self, id: EdgeId) -> bool {
        if let Some(edge) = self.edges.remove(&id) {
            if let Some(out) = self.out_edges.get_mut(&edge.source) {
                out.retain(|&e| e != id);
            }
            if let Some(in_) = self.in_edges.get_mut(&edge.target) {
                in_.retain(|&e| e != id);
            }
            true
        } else {
            false
        }
    }

    /// Get outgoing edges of a node
    ///
    /// Returns all edges where this node is the source.
    pub fn get_outgoing_edges(&self, node_id: NodeId) -> Vec<&GraphEdge> {
        self.out_edges
            .get(&node_id)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get incoming edges of a node
    ///
    /// Returns all edges where this node is the target.
    pub fn get_incoming_edges(&self, node_id: NodeId) -> Vec<&GraphEdge> {
        self.in_edges
            .get(&node_id)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find nodes by label
    ///
    /// Returns all nodes that have the specified label.
    pub fn find_nodes_by_label(&self, label: &str) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|n| n.labels.iter().any(|l| l == label))
            .collect()
    }

    /// Find edges by label
    ///
    /// Returns all edges with the specified label.
    pub fn find_edges_by_label(&self, label: &str) -> Vec<&GraphEdge> {
        self.edges.values().filter(|e| e.label == label).collect()
    }

    /// Traverse from a node following edges with a given label
    ///
    /// Returns all direct neighbors reachable via edges with the specified label
    /// and direction.
    pub fn traverse(
        &self,
        start_id: NodeId,
        edge_label: &str,
        direction: TraversalDirection,
    ) -> Vec<&GraphNode> {
        if !self.nodes.contains_key(&start_id) {
            return Vec::new();
        }

        let mut result = Vec::new();

        match direction {
            TraversalDirection::Outgoing => {
                if let Some(ids) = self.out_edges.get(&start_id) {
                    for &eid in ids {
                        if let Some(edge) = self.edges.get(&eid) {
                            if edge.label == edge_label {
                                if let Some(node) = self.nodes.get(&edge.target) {
                                    result.push(node);
                                }
                            }
                        }
                    }
                }
            }
            TraversalDirection::Incoming => {
                if let Some(ids) = self.in_edges.get(&start_id) {
                    for &eid in ids {
                        if let Some(edge) = self.edges.get(&eid) {
                            if edge.label == edge_label {
                                if let Some(node) = self.nodes.get(&edge.source) {
                                    result.push(node);
                                }
                            }
                        }
                    }
                }
            }
            TraversalDirection::Both => {
                if let Some(ids) = self.out_edges.get(&start_id) {
                    for &eid in ids {
                        if let Some(edge) = self.edges.get(&eid) {
                            if edge.label == edge_label {
                                if let Some(node) = self.nodes.get(&edge.target) {
                                    result.push(node);
                                }
                            }
                        }
                    }
                }
                if let Some(ids) = self.in_edges.get(&start_id) {
                    for &eid in ids {
                        if let Some(edge) = self.edges.get(&eid) {
                            if edge.label == edge_label {
                                if let Some(node) = self.nodes.get(&edge.source) {
                                    result.push(node);
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// Execute a simple graph query using BFS traversal
    ///
    /// Finds nodes matching the start labels, then traverses the graph
    /// up to `max_depth` following edges in the specified direction,
    /// optionally filtered by edge label and node properties.
    pub fn query(&self, query: &GraphQuery) -> GraphQueryResult {
        let start_nodes: Vec<NodeId> = self
            .nodes
            .values()
            .filter(|n| query.start_labels.iter().any(|l| n.labels.contains(l)))
            .map(|n| n.id)
            .collect();

        let mut visited = HashSet::new();
        let mut result_nodes = Vec::new();
        let mut result_edges = Vec::new();
        let mut result_paths = Vec::new();

        let mut queue = VecDeque::new();

        for &start_id in &start_nodes {
            let path = vec![start_id];
            visited.insert(start_id);
            queue.push_back((start_id, 0usize, path));
        }

        while let Some((current_id, depth, path)) = queue.pop_front() {
            // Add current node if it matches property filters
            if let Some(node) = self.nodes.get(&current_id) {
                if Self::matches_filters(node, &query.property_filters) {
                    result_nodes.push(node.clone());
                    result_paths.push(path.clone());
                }
            }

            if depth >= query.max_depth {
                continue;
            }

            // Follow edges
            let incident_edges = match query.direction {
                TraversalDirection::Outgoing => self.get_outgoing_edges(current_id),
                TraversalDirection::Incoming => self.get_incoming_edges(current_id),
                TraversalDirection::Both => {
                    let mut both = self.get_outgoing_edges(current_id);
                    both.extend(self.get_incoming_edges(current_id));
                    both
                }
            };

            for edge in incident_edges {
                // Filter by edge label if specified
                if let Some(ref edge_label) = query.edge_label {
                    if edge.label != *edge_label {
                        continue;
                    }
                }

                let neighbor_id = match query.direction {
                    TraversalDirection::Outgoing => edge.target,
                    TraversalDirection::Incoming => edge.source,
                    TraversalDirection::Both => {
                        if edge.source == current_id {
                            edge.target
                        } else {
                            edge.source
                        }
                    }
                };

                // Only traverse to nodes that exist
                if !self.nodes.contains_key(&neighbor_id) {
                    continue;
                }

                // Record the edge when traversed, regardless of whether the
                // neighbor was already visited (handles start-node-to-start-node edges)
                result_edges.push(edge.clone());

                if visited.insert(neighbor_id) {
                    let mut new_path = path.clone();
                    new_path.push(neighbor_id);
                    queue.push_back((neighbor_id, depth + 1, new_path));
                }
            }
        }

        GraphQueryResult {
            nodes: result_nodes,
            edges: result_edges,
            paths: result_paths,
        }
    }

    /// Check if a node matches all property filter conditions
    fn matches_filters(node: &GraphNode, filters: &HashMap<String, serde_json::Value>) -> bool {
        for (key, value) in filters {
            match node.properties.get(key) {
                Some(prop_val) => {
                    if prop_val != value {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    /// Get all nodes in the graph
    pub fn all_nodes(&self) -> Vec<&GraphNode> {
        self.nodes.values().collect()
    }

    /// Get all edges in the graph
    pub fn all_edges(&self) -> Vec<&GraphEdge> {
        self.edges.values().collect()
    }

    /// Number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for GraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Traversal direction for edge navigation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TraversalDirection {
    /// Follow edges from source to target
    Outgoing,
    /// Follow edges from target to source
    Incoming,
    /// Follow edges in both directions
    Both,
}

/// A simple graph query
///
/// Used with [`GraphEngine::query`] to perform BFS-based graph traversal
/// with filtering by node labels, edge labels, and node properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Only start traversal from nodes with any of these labels
    pub start_labels: Vec<String>,
    /// If set, only traverse edges with this label
    pub edge_label: Option<String>,
    /// Direction to follow edges during traversal
    pub direction: TraversalDirection,
    /// Maximum traversal depth (0 means only start nodes)
    pub max_depth: usize,
    /// Only include nodes whose properties match all entries (key must exist and value must match)
    pub property_filters: HashMap<String, serde_json::Value>,
}

/// Result of a graph query
///
/// Contains all nodes, edges, and paths discovered during query execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    /// Nodes matching the query criteria
    pub nodes: Vec<GraphNode>,
    /// Edges traversed during the query
    pub edges: Vec<GraphEdge>,
    /// Paths from start nodes to discovered nodes (each path is a sequence of node IDs)
    pub paths: Vec<Vec<NodeId>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_props(pairs: Vec<(&str, serde_json::Value)>) -> HashMap<String, serde_json::Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn test_add_and_get_node() {
        let mut graph = GraphEngine::new();
        let id = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );
        let node = graph.get_node(id).unwrap();
        assert_eq!(node.id, id);
        assert_eq!(node.labels, vec!["Person"]);
        assert_eq!(node.properties["name"], serde_json::json!("Alice"));
    }

    #[test]
    fn test_add_edge() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let eid = graph
            .add_edge(a, b, "connected".to_string(), HashMap::new())
            .unwrap();
        let edge = graph.get_edge(eid).unwrap();
        assert_eq!(edge.source, a);
        assert_eq!(edge.target, b);
        assert_eq!(edge.label, "connected");
    }

    #[test]
    fn test_remove_node_cascades() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        graph
            .add_edge(a, b, "link".to_string(), HashMap::new())
            .unwrap();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        assert!(graph.remove_node(a));
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.get_node(a).is_none());
        assert!(graph.get_node(b).is_some());
    }

    #[test]
    fn test_remove_edge() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let eid = graph
            .add_edge(a, b, "link".to_string(), HashMap::new())
            .unwrap();
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.remove_edge(eid));
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.get_edge(eid).is_none());
        // Nodes should still exist
        assert!(graph.get_node(a).is_some());
        assert!(graph.get_node(b).is_some());
    }

    #[test]
    fn test_find_by_label() {
        let mut graph = GraphEngine::new();
        graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );
        graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Bob"))]),
        );
        graph.add_node(
            vec!["Company".to_string()],
            make_props(vec![("name", serde_json::json!("Acme"))]),
        );

        let people = graph.find_nodes_by_label("Person");
        assert_eq!(people.len(), 2);

        let companies = graph.find_nodes_by_label("Company");
        assert_eq!(companies.len(), 1);

        let nonexistent = graph.find_nodes_by_label("Nonexistent");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn test_traverse_outgoing() {
        let mut graph = GraphEngine::new();
        let alice = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );
        let bob = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Bob"))]),
        );
        let carol = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Carol"))]),
        );

        graph
            .add_edge(alice, bob, "knows".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(alice, carol, "knows".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(bob, carol, "knows".to_string(), HashMap::new())
            .unwrap();

        let friends = graph.traverse(alice, "knows", TraversalDirection::Outgoing);
        assert_eq!(friends.len(), 2);
        let names: Vec<&str> = friends
            .iter()
            .map(|n| n.properties["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Bob"));
        assert!(names.contains(&"Carol"));
    }

    #[test]
    fn test_traverse_incoming() {
        let mut graph = GraphEngine::new();
        let alice = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );
        let bob = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Bob"))]),
        );
        graph
            .add_edge(alice, bob, "knows".to_string(), HashMap::new())
            .unwrap();

        let result = graph.traverse(bob, "knows", TraversalDirection::Incoming);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].properties["name"], serde_json::json!("Alice"));
    }

    #[test]
    fn test_edge_to_nonexistent_node_errors() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());

        let result = graph.add_edge(a, 999, "link".to_string(), HashMap::new());
        assert!(result.is_err());

        let result = graph.add_edge(999, a, "link".to_string(), HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_graph() {
        let graph = GraphEngine::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.all_nodes().is_empty());
        assert!(graph.all_edges().is_empty());
    }

    #[test]
    fn test_query_by_label() {
        let mut graph = GraphEngine::new();
        let alice = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );
        let bob = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Bob"))]),
        );
        let charlie = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Charlie"))]),
        );

        graph
            .add_edge(alice, bob, "knows".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(bob, charlie, "knows".to_string(), HashMap::new())
            .unwrap();

        let query = GraphQuery {
            start_labels: vec!["Person".to_string()],
            edge_label: Some("knows".to_string()),
            direction: TraversalDirection::Outgoing,
            max_depth: 2,
            property_filters: HashMap::new(),
        };

        let result = graph.query(&query);
        // Should find Alice (depth 0), Bob (depth 1), Charlie (depth 2)
        assert_eq!(result.nodes.len(), 3);
        // Should have traversed 2 edges
        assert_eq!(result.edges.len(), 2);
    }

    #[test]
    fn test_multiple_labels() {
        let mut graph = GraphEngine::new();
        let id = graph.add_node(
            vec!["Person".to_string(), "Employee".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );

        let people = graph.find_nodes_by_label("Person");
        assert_eq!(people.len(), 1);

        let employees = graph.find_nodes_by_label("Employee");
        assert_eq!(employees.len(), 1);

        assert_eq!(people[0].id, id);
        assert_eq!(employees[0].id, id);
    }

    #[test]
    fn test_node_properties() {
        let mut graph = GraphEngine::new();
        let id = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![
                ("name", serde_json::json!("Alice")),
                ("age", serde_json::json!(30)),
                ("active", serde_json::json!(true)),
            ]),
        );

        let node = graph.get_node(id).unwrap();
        assert_eq!(node.properties["name"], serde_json::json!("Alice"));
        assert_eq!(node.properties["age"], serde_json::json!(30));
        assert_eq!(node.properties["active"], serde_json::json!(true));
    }

    #[test]
    fn test_edge_properties() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let eid = graph
            .add_edge(
                a,
                b,
                "works_at".to_string(),
                make_props(vec![
                    ("since", serde_json::json!(2020)),
                    ("role", serde_json::json!("Engineer")),
                ]),
            )
            .unwrap();

        let edge = graph.get_edge(eid).unwrap();
        assert_eq!(edge.properties["since"], serde_json::json!(2020));
        assert_eq!(edge.properties["role"], serde_json::json!("Engineer"));
    }

    #[test]
    fn test_traverse_both_directions() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["Node".to_string()], HashMap::new());
        let b = graph.add_node(vec!["Node".to_string()], HashMap::new());
        graph
            .add_edge(a, b, "linked".to_string(), HashMap::new())
            .unwrap();

        // Traverse outgoing from a -> should find b
        let outgoing = graph.traverse(a, "linked", TraversalDirection::Outgoing);
        assert_eq!(outgoing.len(), 1);

        // Traverse incoming from a -> should find nothing
        let incoming = graph.traverse(a, "linked", TraversalDirection::Incoming);
        assert!(incoming.is_empty());

        // Traverse both from a -> should find b
        let both = graph.traverse(a, "linked", TraversalDirection::Both);
        assert_eq!(both.len(), 1);

        // Traverse both from b -> should find a
        let both_b = graph.traverse(b, "linked", TraversalDirection::Both);
        assert_eq!(both_b.len(), 1);
    }

    #[test]
    fn test_self_loop_edge() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(
            vec!["Node".to_string()],
            make_props(vec![("name", serde_json::json!("self"))]),
        );
        let eid = graph
            .add_edge(a, a, "self_loop".to_string(), HashMap::new())
            .unwrap();

        let edge = graph.get_edge(eid).unwrap();
        assert_eq!(edge.source, a);
        assert_eq!(edge.target, a);

        let outgoing = graph.traverse(a, "self_loop", TraversalDirection::Outgoing);
        assert_eq!(outgoing.len(), 1);

        let incoming = graph.traverse(a, "self_loop", TraversalDirection::Incoming);
        assert_eq!(incoming.len(), 1);

        let both = graph.traverse(a, "self_loop", TraversalDirection::Both);
        // Both directions: outgoing gives a, incoming gives a -> but both are the same node
        // Since traverse uses a Vec without dedup, Both will return the same node twice
        // This is acceptable behavior - the edge is traversed in each direction separately
        assert_eq!(both.len(), 2);
    }

    #[test]
    fn test_remove_nonexistent_node() {
        let mut graph = GraphEngine::new();
        assert!(!graph.remove_node(999));
    }

    #[test]
    fn test_remove_nonexistent_edge() {
        let mut graph = GraphEngine::new();
        assert!(!graph.remove_edge(999));
    }

    #[test]
    fn test_outgoing_edges_index() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let c = graph.add_node(vec!["C".to_string()], HashMap::new());

        let e1 = graph
            .add_edge(a, b, "to_b".to_string(), HashMap::new())
            .unwrap();
        let e2 = graph
            .add_edge(a, c, "to_c".to_string(), HashMap::new())
            .unwrap();

        let outgoing = graph.get_outgoing_edges(a);
        assert_eq!(outgoing.len(), 2);
        let ids: Vec<EdgeId> = outgoing.iter().map(|e| e.id).collect();
        assert!(ids.contains(&e1));
        assert!(ids.contains(&e2));
    }

    #[test]
    fn test_incoming_edges_index() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let c = graph.add_node(vec!["C".to_string()], HashMap::new());

        let e1 = graph
            .add_edge(a, c, "to_c".to_string(), HashMap::new())
            .unwrap();
        let e2 = graph
            .add_edge(b, c, "also_to_c".to_string(), HashMap::new())
            .unwrap();

        let incoming = graph.get_incoming_edges(c);
        assert_eq!(incoming.len(), 2);
        let ids: Vec<EdgeId> = incoming.iter().map(|e| e.id).collect();
        assert!(ids.contains(&e1));
        assert!(ids.contains(&e2));
    }

    #[test]
    fn test_query_with_property_filter() {
        let mut graph = GraphEngine::new();
        graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![
                ("name", serde_json::json!("Alice")),
                ("age", serde_json::json!(30)),
            ]),
        );
        graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![
                ("name", serde_json::json!("Bob")),
                ("age", serde_json::json!(25)),
            ]),
        );
        graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![
                ("name", serde_json::json!("Charlie")),
                ("age", serde_json::json!(30)),
            ]),
        );

        let query = GraphQuery {
            start_labels: vec!["Person".to_string()],
            edge_label: None,
            direction: TraversalDirection::Outgoing,
            max_depth: 0,
            property_filters: make_props(vec![("age", serde_json::json!(30))]),
        };

        let result = graph.query(&query);
        assert_eq!(result.nodes.len(), 2);
    }

    #[test]
    fn test_query_max_depth_zero() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        graph
            .add_edge(a, b, "link".to_string(), HashMap::new())
            .unwrap();

        let query = GraphQuery {
            start_labels: vec!["A".to_string()],
            edge_label: None,
            direction: TraversalDirection::Outgoing,
            max_depth: 0,
            property_filters: HashMap::new(),
        };

        let result = graph.query(&query);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].id, a);
        assert!(result.edges.is_empty());
    }

    #[test]
    fn test_find_edges_by_label() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let c = graph.add_node(vec!["C".to_string()], HashMap::new());

        graph
            .add_edge(a, b, "knows".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(b, c, "knows".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(a, c, "likes".to_string(), HashMap::new())
            .unwrap();

        let knows = graph.find_edges_by_label("knows");
        assert_eq!(knows.len(), 2);

        let likes = graph.find_edges_by_label("likes");
        assert_eq!(likes.len(), 1);

        let nonexistent = graph.find_edges_by_label("nonexistent");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn test_traverse_with_different_edge_labels() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        graph
            .add_edge(a, b, "knows".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(a, b, "likes".to_string(), HashMap::new())
            .unwrap();

        let knows = graph.traverse(a, "knows", TraversalDirection::Outgoing);
        assert_eq!(knows.len(), 1);

        let likes = graph.traverse(a, "likes", TraversalDirection::Outgoing);
        assert_eq!(likes.len(), 1);
    }

    #[test]
    fn test_remove_node_cleans_indexes() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(vec!["A".to_string()], HashMap::new());
        let b = graph.add_node(vec!["B".to_string()], HashMap::new());
        let c = graph.add_node(vec!["C".to_string()], HashMap::new());

        graph
            .add_edge(a, b, "link".to_string(), HashMap::new())
            .unwrap();
        graph
            .add_edge(b, c, "link".to_string(), HashMap::new())
            .unwrap();

        graph.remove_node(b);

        // Node B is gone
        assert!(graph.get_node(b).is_none());

        // Edges involving B are gone
        assert_eq!(graph.edge_count(), 0);

        // No dangling references in adjacency indexes of A and C
        assert!(graph.get_outgoing_edges(a).is_empty());
        assert!(graph.get_incoming_edges(c).is_empty());
    }

    #[test]
    fn test_large_graph_traversal() {
        let mut graph = GraphEngine::new();
        // Create a chain: 0 -> 1 -> 2 -> ... -> 9
        // Use a distinct label for the first node to limit the BFS seed set
        let first = graph.add_node(
            vec!["Root".to_string()],
            make_props(vec![("index", serde_json::json!(0))]),
        );
        let mut rest = Vec::new();
        for i in 1..10 {
            rest.push(graph.add_node(
                vec!["Node".to_string()],
                make_props(vec![("index", serde_json::json!(i))]),
            ));
        }
        let mut prev = first;
        for &next in &rest {
            graph
                .add_edge(prev, next, "next".to_string(), HashMap::new())
                .unwrap();
            prev = next;
        }

        // BFS from the single "Root" node with depth 3
        let query = GraphQuery {
            start_labels: vec!["Root".to_string()],
            edge_label: Some("next".to_string()),
            direction: TraversalDirection::Outgoing,
            max_depth: 3,
            property_filters: HashMap::new(),
        };

        let result = graph.query(&query);
        // Should find 4 nodes (Root + 3 traversed), and 3 edges
        assert_eq!(result.nodes.len(), 4);
        assert_eq!(result.edges.len(), 3);
    }

    #[test]
    fn test_graph_serialization_roundtrip() {
        let mut graph = GraphEngine::new();
        let a = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Alice"))]),
        );
        let b = graph.add_node(
            vec!["Person".to_string()],
            make_props(vec![("name", serde_json::json!("Bob"))]),
        );
        graph
            .add_edge(a, b, "knows".to_string(), HashMap::new())
            .unwrap();

        let serialized = serde_json::to_string(&graph).unwrap();
        let deserialized: GraphEngine = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.node_count(), 2);
        assert_eq!(deserialized.edge_count(), 1);
        assert!(deserialized.get_node(a).is_some());
        assert!(deserialized.get_node(b).is_some());
    }
}
