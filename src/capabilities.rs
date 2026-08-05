/*!
# Server Capabilities — the negotiation contract

`GET /api/v1/capabilities` (and this module) describe what a PrimusDB node
can actually do, so clients and drivers never hardcode expectations.

The capabilities are derived from the **capability registry** (self-discovery
architecture): engines register their `list_tables`, and each registered engine
appears in `engines` automatically. New engines show up without touching the
CLI, REPL or drivers.

Consumers:

- **REPL** (`primusdb shell`) fetches server metadata (version, node id,
  engine/table lists) from this endpoint for its banner and Tab completion.
- **Drivers** negotiate capabilities: they check which engines/features are
  present *before* sending a query, and can warn or fail when the server does
  not support what the application needs.

## Contract

```json
{
  "protocol_version": 1,
  "server": {
    "version": "1.3.2-alpha",
    "node_id": "...",
    "instance_id": "...",
    "uptime_seconds": 123
  },
  "engines": [
    { "storage_type": "Relational", "tables": ["users", "orders"] }
  ],
  "features": ["search", "graphql", "integrity", "ledger", "fulltext", ...]
}
```

Features are additive: a client should never fail on unknown features, only on
missing ones it requires.
*/

use serde::{Deserialize, Serialize};

/// Full capability snapshot of a PrimusDB node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Negotiation protocol version. Bump on breaking shape changes.
    pub protocol_version: u32,
    /// Static identity and runtime info of the node.
    pub server: ServerInfo,
    /// Per-engine capabilities discovered through the capability registry.
    pub engines: Vec<EngineCapabilities>,
    /// Additive feature flags the node supports.
    pub features: Vec<String>,
}

/// Node identity and runtime information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub node_id: String,
    pub instance_id: String,
    pub uptime_seconds: u64,
}

/// Capabilities of a single storage engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilities {
    /// Engine type name, e.g. `Relational` (matches `StorageType` Display).
    pub storage_type: String,
    /// Tables/collections/indexes currently present in the engine.
    pub tables: Vec<String>,
}

/// Protocol version of this capabilities handshake.
pub const PROTOCOL_VERSION: u32 = 1;

/// Stable, additive feature flags advertised by every node.
pub fn default_features() -> Vec<String> {
    vec![
        "search".to_string(),
        "graphql".to_string(),
        "integrity".to_string(),
        "ledger".to_string(),
        "fulltext".to_string(),
        "vector_search".to_string(),
        "capability_negotiation".to_string(),
    ]
}
