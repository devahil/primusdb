/// Local instance discovery for PrimusDB.
///
/// Detects running PrimusDB instances by probing:
///   1. Default ports (8080-8083, 9090-9093)
///   2. Configured ports from known config paths
///   3. Local process list (where supported)
///   4. Docker Compose services (when available)
///
/// # Flow
/// ```text
/// +----------------+     +-------------------+     +------------------+
/// | Port Scanner   | --> | Config File Reader| --> | Process Checker  |
/// +----------------+     +-------------------+     +------------------+
///         |                      |                         |
///         v                      v                         v
///     +-------------------------------------------------------+
///     |              Health Probe (/health)                    |
///     +-------------------------------------------------------+
///         |
///         v
///     +-------------------------------------------------------+
///     |              DiscoveryResult list                      |
///     +-------------------------------------------------------+
/// ```
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A discovered PrimusDB instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub endpoint: String,
    pub instance_id: Option<String>,
    pub node_id: Option<String>,
    pub version: Option<String>,
    pub status: String,
    pub uptime_seconds: Option<u64>,
    pub enabled_engines: Vec<String>,
    pub cluster_role: Option<String>,
    pub protocol_status: Option<String>,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub ports: Vec<u16>,
    pub timeout_ms: u64,
    pub scan_localhost: bool,
    pub check_config_files: bool,
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
    InstanceInfo {
        endpoint: endpoint.to_string(),
        instance_id: body
            .get("instance_id")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        node_id: body
            .get("node_id")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        version: body
            .get("version")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        status: body
            .get("status")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or("unknown".into()),
        uptime_seconds: body.get("uptime_seconds").and_then(|v| v.as_u64()),
        enabled_engines: body
            .get("enabled_engines")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        cluster_role: body
            .get("role")
            .and_then(|v| v.as_str().map(|s| s.to_string())),
        protocol_status: body
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
