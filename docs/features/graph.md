# Graph Engine

> **NOT AVAILABLE**: The graph engine described here lives in `src/graph.rs`,
> which is not declared in `src/lib.rs` and is **not compiled** in the current
> build. The `primusdb graph` CLI subcommand exists but responds that graph
> operations are not yet available via the CLI. This page is kept as reference
> for the intended design.

## Overview

PrimusDB includes an in-memory property graph database engine that supports the
property graph model with nodes and directed edges, labels, and key-value
properties. The engine is designed for high-performance graph traversal, pattern
matching, and relationship-oriented queries within the PrimusDB ecosystem.

The graph engine is defined in `src/graph.rs` and provides:

- Nodes with multiple labels and arbitrary JSON properties
- Directed edges with labels and properties
- Automatic adjacency indexing (outgoing/incoming edges)
- BFS and DFS traversal with direction and label filtering
- Simple graph query execution
- Full serialization/deserialization via Serde

---

## Architecture

### Property Graph Model

The graph engine implements the **property graph model**, a widely adopted
paradigm in graph databases (used by Neo4j, Apache TinkerPop, etc.). In this
model:

- **Nodes** represent entities (people, places, things)
- **Edges** represent directed relationships between nodes
- Both nodes and edges carry **labels** (types) and **properties** (key-value
  data)

```
  ┌──────────────────┐          ┌──────────────────┐
  │     GraphNode    │          │     GraphEdge    │
  ├──────────────────┤          ├──────────────────┤
  │ id: NodeId       │          │ id: EdgeId       │
  │ labels: Vec<String>   │───>  │ source: NodeId   │
  │ properties: Map  │          │ target: NodeId   │
  └──────────────────┘          │ label: String    │
                                │ properties: Map  │
                                └──────────────────┘
```

### Data Structures

The `GraphEngine` struct is the central data structure:

```rust
pub struct GraphEngine {
    nodes: HashMap<NodeId, GraphNode>,
    edges: HashMap<EdgeId, GraphEdge>,
    out_edges: HashMap<NodeId, Vec<EdgeId>>,
    in_edges: HashMap<NodeId, Vec<EdgeId>>,
    next_node_id: NodeId,
    next_edge_id: EdgeId,
}
```

- `nodes` / `edges` — Primary storage of all graph elements
- `out_edges` — Adjacency index: for each node, a list of outgoing edge IDs
- `in_edges` — Adjacency index: for each node, a list of incoming edge IDs
- `next_node_id` / `next_edge_id` — Auto-incrementing ID counters

### Adjacency Indexing

The engine maintains **dual adjacency indexes** (`out_edges` and `in_edges`) for
O(1) lookups of incident edges. When a directed edge is added from node A to
node B:

1. The edge is stored in `edges` keyed by its `EdgeId`
2. The edge ID is appended to `out_edges[A]`
3. The edge ID is appended to `in_edges[B]`

This design enables efficient traversal in any direction without scanning all
edges. When a node is removed, all incident edges are automatically cleaned up
from both indexes.

---

## Node and Edge Structures

### GraphNode

```rust
pub struct GraphNode {
    pub id: NodeId,
    pub labels: Vec<String>,
    pub properties: HashMap<String, serde_json::Value>,
}
```

| Field        | Type                               | Description                            |
|--------------|------------------------------------|----------------------------------------|
| `id`         | `NodeId` (`u64`)                   | Auto-generated unique identifier       |
| `labels`     | `Vec<String>`                      | Type labels (e.g., `"Person"`, `"Company"`) |
| `properties` | `HashMap<String, serde_json::Value>` | Arbitrary key-value data as JSON values |

Nodes can have **multiple labels** simultaneously (e.g., a node can be both
`"Person"` and `"Employee"`). Labels are used for filtering nodes during
queries and traversals.

Properties support any JSON-compatible value: strings, numbers, booleans,
arrays, nested objects, or null.

### GraphEdge

```rust
pub struct GraphEdge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
}
```

| Field        | Type                               | Description                              |
|--------------|------------------------------------|------------------------------------------|
| `id`         | `EdgeId` (`u64`)                   | Auto-generated unique identifier         |
| `source`     | `NodeId`                           | ID of the source/origin node             |
| `target`     | `NodeId`                           | ID of the destination node               |
| `label`      | `String`                           | Relationship type (e.g., `"knows"`, `"works_at"`) |
| `properties` | `HashMap<String, serde_json::Value>` | Arbitrary key-value data as JSON values   |

Edges are **directed** (source → target). Self-loop edges (where source ==
target) are supported. Edge labels describe the relationship type and are the
primary mechanism for filtering during traversal.

### Type Aliases

```rust
pub type NodeId = u64;
pub type EdgeId = u64;
```

---

## Core API

### Creating Nodes

```rust
let alice = graph.add_node(
    vec!["Person".to_string()],
    HashMap::from([("name".to_string(), serde_json::json!("Alice"))]),
);
```

`add_node` takes a vector of labels and a property map, and returns the
auto-generated `NodeId`.

### Creating Edges

```rust
graph.add_edge(alice, bob, "knows".to_string(), HashMap::new()).unwrap();
```

`add_edge` takes source ID, target ID, a label, and a property map. Returns
`Err(String)` if either node does not exist.

### Lookup Methods

| Method                      | Description                              |
|-----------------------------|------------------------------------------|
| `get_node(id)`              | Lookup a node by ID                      |
| `get_edge(id)`              | Lookup an edge by ID                     |
| `find_nodes_by_label(label)` | Find all nodes with a given label        |
| `find_edges_by_label(label)` | Find all edges with a given label        |
| `get_outgoing_edges(node_id)` | Get all edges where node is the source |
| `get_incoming_edges(node_id)` | Get all edges where node is the target |
| `all_nodes()`               | Return all nodes                          |
| `all_edges()`               | Return all edges                          |
| `node_count()`              | Number of nodes in the graph              |
| `edge_count()`              | Number of edges in the graph              |

### Removal Operations

| Method                 | Description                                          |
|------------------------|------------------------------------------------------|
| `remove_node(id)`      | Remove a node and all incident edges (cascade)       |
| `remove_edge(id)`      | Remove a single edge; nodes are preserved             |

When a node is removed, the engine automatically:
1. Collects all outgoing and incoming edge IDs
2. Removes every edge from the `edges` map
3. Removes dangling references from neighbor adjacency indexes
4. Removes the node itself

---

## Traversal Strategies

### Direction-Based Traversal

The `Traverse` method provides direct neighbor traversal filtered by edge label
and direction:

```rust
pub fn traverse(
    &self,
    start_id: NodeId,
    edge_label: &str,
    direction: TraversalDirection,
) -> Vec<&GraphNode>
```

#### TraversalDirection

```rust
pub enum TraversalDirection {
    Outgoing,  // Follow edges source → target
    Incoming,  // Follow edges target → source
    Both,      // Follow edges in both directions
}
```

- **Outgoing**: Starting from node A, returns all nodes reachable via outgoing
  edges with the matching label.
- **Incoming**: Starting from node A, returns all nodes that have edges
  pointing to A with the matching label.
- **Both**: Traverses edges in both directions, returning neighbors on either
  side.

### Query-Based Traversal (BFS)

The `GraphQuery` system provides multi-level BFS traversal with filtering:

```rust
pub struct GraphQuery {
    pub start_labels: Vec<String>,
    pub edge_label: Option<String>,
    pub direction: TraversalDirection,
    pub max_depth: usize,
    pub property_filters: HashMap<String, serde_json::Value>,
}
```

The `query` method:

1. **Seeds** the BFS queue with all nodes matching `start_labels`
2. **Visits** nodes level by level, up to `max_depth`
3. **Filters** edges by `edge_label` (if set)
4. **Filters** nodes by `property_filters` (all specified key-value pairs must
   match)
5. **Tracks** visited nodes to avoid cycles
6. Returns a `GraphQueryResult` containing discovered nodes, traversed edges,
   and full paths

```rust
pub struct GraphQueryResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub paths: Vec<Vec<NodeId>>,
}
```

### BFS (Breadth-First Search)

The built-in `query` method uses BFS. It explores the graph level by level:

- Depth 0: start nodes (matching `start_labels`)
- Depth 1: direct neighbors reachable via matching edges
- Depth N: nodes at N steps from a start node

BFS is ideal for:
- Finding shortest paths
- Social network "friends of friends" queries
- Recommendation systems (similar items within N hops)
- Proximity analysis

### DFS (Depth-First Search)

DFS traversal is available through the CLI `graph traverse` command with
`--strategy dfs`. DFS explores as far as possible along each branch before
backtracking.

DFS is ideal for:
- Path existence queries
- Topological sorting
- Detecting cycles
- Exploring deep, narrow graphs

---

## CLI Commands

Graph operations are available through the `primusdb graph` CLI subcommand. The
handler is implemented in `src/cli/cmd/graph.rs` with command definitions in
`src/cli/command.rs`.

### `primusdb graph nodes`

Query and list graph nodes.

**Usage:**
```
primusdb graph nodes [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --label <LABEL>` | Filter by node label | — |
| `-f, --filter <EXPR>` | Property filter expression | — |
| `--limit <N>` | Maximum nodes to return | `100` |
| `--counts` | Return counts only | `false` |

**Examples:**
```bash
primusdb graph nodes
primusdb graph nodes --label Person --limit 50
primusdb graph nodes --label Product --filter "price > 100" --counts
```

### `primusdb graph edges`

Query and list graph edges.

**Usage:**
```
primusdb graph edges [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --from <NODE>` | Filter by source node | — |
| `-t, --to <NODE>` | Filter by target node | — |
| `-l, --label <LABEL>` | Filter by edge label | — |
| `--limit <N>` | Maximum edges to return | `100` |

**Examples:**
```bash
primusdb graph edges
primusdb graph edges --label PURCHASED --limit 50
primusdb graph edges --from user_123 --label FOLLOWS
```

### `primusdb graph query`

Execute a graph query using a graph query language.

**Usage:**
```
primusdb graph query <QUERY> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `QUERY` | Graph query string (space-separated) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-l, --language <LANG>` | Query language | `cypher` |

**Supported languages:** `cypher`, `gremlin`, `sparql`

**Examples:**
```bash
primusdb graph query "MATCH (p:Person)-[:FRIENDS]->(f) RETURN p.name, f.name"
primusdb graph query "g.V().hasLabel('Person').out('FRIENDS')" --language gremlin
```

### `primusdb graph traverse`

Traverse the graph from a starting node.

**Usage:**
```
primusdb graph traverse <START> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `START` | Starting node ID |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --depth <N>` | Maximum traversal depth | `3` |
| `-l, --label <LABEL>` | Edge label to follow | — |
| `-s, --strategy <STRATEGY>` | Traversal strategy | `bfs` |

**Strategies:** `bfs` (breadth-first), `dfs` (depth-first)

**Examples:**
```bash
primusdb graph traverse user_123 --depth 2 --label FRIENDS
primusdb graph traverse root_node --depth 5 --strategy dfs
```

---

## Serialization

The entire `GraphEngine`, including `GraphNode`, `GraphEdge`, and both
adjacency indexes, implements `Serialize` and `Deserialize` from Serde. This
allows the graph to be persisted and restored as JSON:

```rust
let serialized = serde_json::to_string(&graph).unwrap();
let deserialized: GraphEngine = serde_json::from_str(&serialized).unwrap();
```

Round-trip serialization preserves all data including node/edge IDs, labels,
properties, and index integrity.

---

## Use Cases

### Social Graphs

Model social networks with `Person` nodes connected by `FOLLOWS`, `FRIENDS`, or
`LIKES` edges:

```rust
let alice = graph.add_node(
    vec!["Person".to_string()],
    HashMap::from([("name".to_string(), serde_json::json!("Alice"))]),
);
let bob = graph.add_node(
    vec!["Person".to_string()],
    HashMap::from([("name".to_string(), serde_json::json!("Bob"))]),
);
graph.add_edge(alice, bob, "FOLLOWS".to_string(), HashMap::new()).unwrap();
```

Traverse to find "friends of friends":

```rust
let query = GraphQuery {
    start_labels: vec!["Person".to_string()],
    edge_label: Some("FOLLOWS".to_string()),
    direction: TraversalDirection::Outgoing,
    max_depth: 2,
    property_filters: HashMap::new(),
};
let result = graph.query(&query);
```

### Knowledge Graphs

Represent entities and their relationships as a knowledge graph. For example, a
movie knowledge graph with `Person`, `Movie`, and `Genre` nodes:

| Node Label | Properties                    |
|------------|-------------------------------|
| `Person`   | `name`, `birth_year`          |
| `Movie`    | `title`, `release_year`, `rating` |
| `Genre`    | `name`                        |

Edges connect these: `ACTED_IN`, `DIRECTED`, `BELONGS_TO_GENRE`.

```rust
let movie = graph.add_node(
    vec!["Movie".to_string()],
    HashMap::from([
        ("title".to_string(), serde_json::json!("Inception")),
        ("rating".to_string(), serde_json::json!(8.8)),
    ]),
);
let genre = graph.add_node(
    vec!["Genre".to_string()],
    HashMap::from([("name".to_string(), serde_json::json!("Sci-Fi"))]),
);
graph.add_edge(movie, genre, "BELONGS_TO_GENRE".to_string(), HashMap::new()).unwrap();
```

Query: "Find all Sci-Fi movies with rating > 8.0":

```rust
let query = GraphQuery {
    start_labels: vec!["Genre".to_string()],
    edge_label: Some("BELONGS_TO_GENRE".to_string()),
    direction: TraversalDirection::Incoming,
    max_depth: 1,
    property_filters: HashMap::from([
        ("name".to_string(), serde_json::json!("Sci-Fi")),
    ]),
};
```

### Recommendation Systems

Use graph traversals to power recommendation engines. For an e-commerce
platform:

- `User` nodes with purchase history
- `Product` nodes with category and price
- `PURCHASED` edges with quantity and date properties
- `IN_CATEGORY` edges linking products to categories

Collaborative filtering via graph traversal — "Users who bought X also bought
Y":

```rust
let query = GraphQuery {
    start_labels: vec!["Product".to_string()],
    edge_label: Some("PURCHASED".to_string()),
    direction: TraversalDirection::Incoming,
    max_depth: 1,  // Find users who bought this product
    property_filters: HashMap::new(),
};
// Then traverse from those users' other purchases
```

### Access Control and Authorization

Model role-based access control as a graph:

- `User` nodes
- `Role` nodes (e.g., "admin", "editor", "viewer")
- `Resource` nodes (e.g., "document:123", "api:reports")
- `HAS_ROLE` edges from User to Role
- `CAN_ACCESS` edges from Role to Resource

A traversal can determine whether a user has access to a resource by checking
reachability through the role hierarchy.

### Network and IT Operations

Model infrastructure as a graph for impact analysis:

- `Server`, `Database`, `Service`, `LoadBalancer` nodes
- `DEPENDS_ON`, `HOSTED_ON`, `ROUTES_TO` edges
- Properties for status, region, version

When a server goes down, traverse to find all affected services:

```rust
let query = GraphQuery {
    start_labels: vec!["Server".to_string()],
    edge_label: Some("HOSTED_ON".to_string()),
    direction: TraversalDirection::Incoming,
    max_depth: 5,
    property_filters: HashMap::from([
        ("status".to_string(), serde_json::json!("degraded")),
    ]),
};
```

---

## Performance Characteristics

| Operation              | Time Complexity       |
|------------------------|-----------------------|
| Add node               | O(1)                  |
| Add edge               | O(1)                  |
| Get node/edge by ID    | O(1)                  |
| Remove node            | O(deg(v)) — proportional to number of incident edges |
| Remove edge            | O(deg(v)) — cleanup in adjacency indexes |
| Find nodes by label    | O(n) — linear scan of all nodes |
| Find edges by label    | O(m) — linear scan of all edges |
| Traverse (1 hop)       | O(deg(v)) — per node adjacency index |
| Query (BFS)            | O(n + m) — visits each node/edge at most once |
| Serialize/Deserialize  | O(n + m) — proportional to total elements |

All operations are in-memory with no disk I/O during traversal. Performance is
bounded by available RAM.

---

## Limitations

- **In-memory only** — The engine holds all data in RAM. Persistent storage
  requires serializing to disk manually.
- **No query planner** — The `query` method uses a simple BFS strategy without
  cost-based optimization.
- **No ACID transactions** — Mutations are applied immediately without
  transactional guarantees.
- **No property indexes** — Label and property filtering requires linear scans
  of all nodes/edges.
- **Simple property filtering** — Filters are exact equality matches only (no
  range queries, regex, or comparison operators).
