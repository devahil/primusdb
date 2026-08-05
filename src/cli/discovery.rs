//! Local instance discovery for PrimusDB.
//!
//! Detects running PrimusDB instances by scanning a set of ports on
//! localhost (default 8080-8083, 9090-9093) and probing each reachable
//! endpoint's health endpoints (`/health`, `/status`, `/protocol/health`)
//! to build an [`InstanceInfo`] for every responding server.
//!
//! # Flow
//! ```text
//! DiscoveryConfig.ports ──► for each port
//!          │
//!          ▼
//!   probe_instance(endpoint)
//!          │  try /health, /status, /protocol/health
//!          ▼
//!   parse_instance (unwrap APIResponse "data" if present)
//!          │
//!          ▼
//!   Vec<InstanceInfo>
//! ```
//!
//! [`DiscoveryConfig`] also exposes `scan_localhost`, `check_config_files`
//! and `check_processes` switches; today `discover_local` always scans
//! `127.0.0.1` and only probes ports — the config-file and process-list
//! strategies are reserved for future work.
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A discovered PrimusDB instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    /// Base endpoint URL of the instance, e.g. `http://127.0.0.1:8080`
    pub endpoint: String,
    /// Instance identifier reported by the server, if any
    pub instance_id: Option<String>,
    /// Node identifier reported by the server, if any
    pub node_id: Option<String>,
    /// Server version string, if reported
    pub version: Option<String>,
    /// Health status, e.g. `healthy` or `unknown`
    pub status: String,
    /// Uptime in seconds, when reported
    pub uptime_seconds: Option<u64>,
    /// Storage engines enabled on the instance
    pub enabled_engines: Vec<String>,
    /// Cluster role (leader, follower, ...) when the instance is clustered
    pub cluster_role: Option<String>,
    /// Protocol layer status, when reported
    pub protocol_status: Option<String>,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Ports to probe on localhost
    pub ports: Vec<u16>,
    /// Per-probe timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether to scan localhost at all
    pub scan_localhost: bool,
    /// Whether to consult known config files for configured ports
    pub check_config_files: bool,
    /// Whether to inspect the local process list (where supported)
    pub check_processes: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            ports: vec![8080, 8081, 8082, 8083, 9090, 9091, 9092, 9093],
            timeout_ms: 500,
            scan_localhost: true,
            check_config_files: true,
            check_processes: true,
        }
    }
}

/// Discover local PrimusDB instances
pub async fn discover_local(config: &DiscoveryConfig) -> Vec<InstanceInfo> {
    let mut instances = Vec::new();

    for port in &config.ports {
        let endpoint = format!("http://127.0.0.1:{}", port);
        match probe_instance(&endpoint, Duration::from_millis(config.timeout_ms)).await {
            Some(info) => instances.push(info),
            None => continue,
        }
    }

    instances
}

/// Probe a single endpoint for health
async fn probe_instance(endpoint: &str, timeout: Duration) -> Option<InstanceInfo> {
    // Try /health endpoint
    let client = reqwest::Client::builder().timeout(timeout).build().ok()?;

    // Try multiple health endpoints
    for path in &["/health", "/status", "/protocol/health"] {
        let url = format!("{}{}", endpoint, path);
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    return Some(parse_instance(endpoint, &body));
                }
            }
        }
    }

    None
}

fn parse_instance(endpoint: &str, body: &serde_json::Value) -> InstanceInfo {
    // Unwrap APIResponse wrapper if present (PrimusDB endpoints return {success, data, ...})
    let data = body
        .get("data")
        .filter(|_| {
            body.get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .unwrap_or(body);

    InstanceInfo {
        endpoint: endpoint.to_string(),
        instance_id: data
            .get("instance_id")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        node_id: data
            .get("node_id")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        version: data
            .get("version")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        status: data
            .get("status")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or("unknown".into()),
        uptime_seconds: data.get("uptime_seconds").and_then(|v| v.as_u64()),
        enabled_engines: data
            .get("enabled_engines")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        cluster_role: data
            .get("role")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        protocol_status: data
            .get("protocol_status")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
    }
}

/// Display discovered instances in a table format
pub fn display_instances(instances: &[InstanceInfo]) {
    if instances.is_empty() {
        println!("No PrimusDB instances found on localhost.");
        println!("Start one with: primusdb server start");
        return;
    }

    println!("Found {} PrimusDB instance(s):", instances.len());
    println!();
    // Print table header
    println!(
        "{:<25} {:<20} {:<10} {:<15}",
        "Endpoint", "Node ID", "Version", "Status"
    );
    println!("{}", "-".repeat(75));
    for inst in instances {
        println!(
            "{:<25} {:<20} {:<10} {:<15}",
            inst.endpoint,
            inst.node_id.as_deref().unwrap_or("-"),
            inst.version.as_deref().unwrap_or("-"),
            inst.status,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instance_full() {
        let json = serde_json::json!({
            "instance_id": "inst-1",
            "node_id": "node-1",
            "version": "1.3.1-alpha",
            "status": "healthy",
            "uptime_seconds": 3600,
            "enabled_engines": ["columnar", "vector"],
            "role": "leader",
            "protocol_status": "active"
        });
        let info = parse_instance("http://127.0.0.1:8080", &json);
        assert_eq!(info.endpoint, "http://127.0.0.1:8080");
        assert_eq!(info.instance_id, Some("inst-1".into()));
        assert_eq!(info.node_id, Some("node-1".into()));
        assert_eq!(info.version, Some("1.3.1-alpha".into()));
        assert_eq!(info.status, "healthy");
        assert_eq!(info.cluster_role, Some("leader".into()));
    }

    #[test]
    fn test_parse_instance_minimal() {
        let json = serde_json::json!({"status": "ok"});
        let info = parse_instance("http://127.0.0.1:8081", &json);
        assert_eq!(info.endpoint, "http://127.0.0.1:8081");
        assert_eq!(info.status, "ok");
        assert!(info.node_id.is_none());
        assert!(info.enabled_engines.is_empty());
    }

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.ports.len(), 8);
        assert!(config.ports.contains(&8080));
        assert_eq!(config.timeout_ms, 500);
    }

    #[test]
    fn test_display_instances_empty() {
        // Should not panic
        display_instances(&[]);
    }

    #[test]
    fn test_display_instances_with_data() {
        let instances = vec![InstanceInfo {
            endpoint: "http://127.0.0.1:8080".into(),
            instance_id: Some("inst-1".into()),
            node_id: Some("node-1".into()),
            version: Some("1.3.0".into()),
            status: "healthy".into(),
            uptime_seconds: Some(3600),
            enabled_engines: vec!["columnar".into()],
            cluster_role: Some("leader".into()),
            protocol_status: Some("active".into()),
        }];
        // Should not panic
        display_instances(&instances);
    }
}
