use crate::cli::discovery::{self, InstanceInfo};
use std::path::Path;
use std::time::Duration;

pub async fn fetch_status(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client.get(format!("{}/status", url)).send().await.ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_health(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client.get(format!("{}/health", url)).send().await.ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_metrics(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp = client.get(format!("{}/metrics", url)).send().await.ok()?;
    if resp.status().is_success() {
        resp.text().await.ok()
    } else {
        None
    }
}

pub async fn fetch_query(url: &str, query: &str) -> String {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("Client error: {}", e),
    };
    let body = serde_json::json!({ "query": query });
    match client
        .post(format!("{}/api/v1/uql", url))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let result: serde_json::Value = resp.json().await.unwrap_or_default();
            serde_json::to_string_pretty(&result).unwrap_or_default()
        }
        Err(e) => format!("Query failed: {}", e),
    }
}

pub async fn fetch_cluster_status(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/cluster/status", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_cluster_nodes(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/cluster/nodes", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_cluster_health(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/cache/cluster/health", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_databases(url: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let resp = match client.get(format!("{}/api/v1/databases", url)).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(obj) = value.as_object() {
        if let Some(dbs) = obj.get("databases").and_then(|d| d.as_array()) {
            dbs.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![serde_json::to_string_pretty(&value).unwrap_or_default()]
        }
    } else {
        Vec::new()
    }
}

pub async fn fetch_namespaces(url: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let resp = match client
        .get(format!("{}/api/v1/namespaces", url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(obj) = value.as_object() {
        if let Some(ns) = obj.get("namespaces").and_then(|d| d.as_array()) {
            ns.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![serde_json::to_string_pretty(&value).unwrap_or_default()]
        }
    } else {
        Vec::new()
    }
}

pub async fn fetch_users(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/auth/users", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_diagnostics(url: &str) -> String {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok();
    let client = match client {
        Some(c) => c,
        None => return "Failed to create HTTP client".to_string(),
    };

    let health_resp = client.get(format!("{}/health", url)).send().await;
    let status_resp = client.get(format!("{}/status", url)).send().await;

    let mut result = String::new();
    result.push_str("=== Health Check ===\n");
    match health_resp {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(v) = resp.json::<serde_json::Value>().await {
                result.push_str(&serde_json::to_string_pretty(&v).unwrap_or_default());
            } else {
                result.push_str("(non-JSON response)\n");
            }
        }
        Ok(resp) => {
            result.push_str(&format!("HTTP {}\n", resp.status()));
        }
        Err(e) => {
            result.push_str(&format!("Error: {}\n", e));
        }
    }
    result.push('\n');
    result.push_str("=== Status ===\n");
    match status_resp {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(v) = resp.json::<serde_json::Value>().await {
                result.push_str(&serde_json::to_string_pretty(&v).unwrap_or_default());
            } else {
                result.push_str("(non-JSON response)\n");
            }
        }
        Ok(resp) => {
            result.push_str(&format!("HTTP {}\n", resp.status()));
        }
        Err(e) => {
            result.push_str(&format!("Error: {}\n", e));
        }
    }

    result
}

pub async fn fetch_settings(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/config", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn run_discovery() -> Vec<InstanceInfo> {
    let config = discovery::DiscoveryConfig::default();
    discovery::discover_local(&config).await
}

pub fn list_backups() -> Vec<String> {
    let backup_dir = Path::new("backups");
    if !backup_dir.exists() || !backup_dir.is_dir() {
        return Vec::new();
    }

    let index_path = backup_dir.join(".index.json");
    if index_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&index_path) {
            if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(backups) = index.get("backups").and_then(|b| b.as_array()) {
                    let mut result: Vec<String> = backups
                        .iter()
                        .map(|entry| {
                            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            let size = entry
                                .get("size_bytes")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let created = entry
                                .get("created_at")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let size_str = if size > 1024 * 1024 {
                                format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                            } else if size > 1024 {
                                format!("{:.1} KB", size as f64 / 1024.0)
                            } else {
                                format!("{} B", size)
                            };
                            format!("{}  {:>8}  {}  {}", "Index", size_str, id, created)
                        })
                        .collect();
                    result.sort();
                    return result;
                }
            }
        }
    }

    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                let name = entry.file_name().to_string_lossy().to_string();
                let kind = if name.ends_with(".sql") {
                    "SQL"
                } else if name.ends_with(".json") {
                    "JSON"
                } else if name.ends_with(".parquet") {
                    "Parquet"
                } else if meta.is_dir() {
                    "Directory"
                } else {
                    "Unknown"
                };
                let size_str = if size > 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else if size > 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{} B", size)
                };
                result.push(format!("{}  {:>8}  {}", kind, size_str, name));
            } else {
                let name = entry.file_name().to_string_lossy().to_string();
                result.push(format!("Unknown           {}", name));
            }
        }
    }
    result.sort();
    result
}

pub async fn fetch_engine_metrics(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/metrics/prometheus", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.text().await.ok()
    } else {
        None
    }
}

pub async fn fetch_cluster_summary(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/cluster/status", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub fn parse_prometheus_metric(data: &str, metric_name: &str) -> Option<String> {
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(metric_name) && !trimmed.starts_with('#') {
            let after = &trimmed[metric_name.len()..];
            let value_str = if after.starts_with('{') {
                if let Some(end) = after.find('}') {
                    after[end + 1..].trim()
                } else {
                    continue;
                }
            } else {
                after.trim()
            };
            if let Some(val_str) = value_str.split_whitespace().next() {
                if let Ok(val) = val_str.parse::<f64>() {
                    if metric_name.contains("memory") || metric_name.contains("bytes") {
                        if val > 1024.0 * 1024.0 * 1024.0 {
                            return Some(format!("{:.2} GB", val / (1024.0 * 1024.0 * 1024.0)));
                        } else if val > 1024.0 * 1024.0 {
                            return Some(format!("{:.2} MB", val / (1024.0 * 1024.0)));
                        } else if val > 1024.0 {
                            return Some(format!("{:.2} KB", val / 1024.0));
                        } else {
                            return Some(format!("{} B", val as u64));
                        }
                    }
                    return Some(format!("{}", val as u64));
                }
            }
        }
    }
    None
}

pub fn list_backups_detail() -> Option<serde_json::Value> {
    let backup_dir = Path::new("backups");
    let index_path = backup_dir.join(".index.json");
    if index_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&index_path) {
            if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                return Some(index);
            }
        }
    }
    None
}

pub async fn fetch_tables(url: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let resp = match client.get(format!("{}/api/v1/tables", url)).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(obj) = value.as_object() {
        if let Some(tables) = obj.get("tables").and_then(|d| d.as_array()) {
            tables
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![serde_json::to_string_pretty(&value).unwrap_or_default()]
        }
    } else {
        Vec::new()
    }
}

pub async fn fetch_vector_indexes(url: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let resp = match client
        .get(format!("{}/api/v1/vector/indexes", url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(obj) = value.as_object() {
        if let Some(idx) = obj.get("indexes").and_then(|d| d.as_array()) {
            idx.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            vec![serde_json::to_string_pretty(&value).unwrap_or_default()]
        }
    } else {
        Vec::new()
    }
}

pub async fn fetch_graph_data(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/graph/status", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_aiml_data(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/ai/models", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_roles(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/auth/roles", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_cluster_events(url: &str) -> Vec<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let resp = match client
        .get(format!("{}/api/v1/cluster/events", url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        vec![serde_json::to_string_pretty(&value).unwrap_or_default()]
    }
}

pub fn fetch_local_logs() -> String {
    fetch_journalctl()
}

pub async fn migrate_inspect_source(source_type: &str, url: &str) -> Result<String, String> {
    if url.is_empty() || !url.contains("://") {
        return Err("Invalid URL format — must contain ://".to_string());
    }
    let valid_sources = ["mysql", "postgresql", "mongodb", "couchdb"];
    if !valid_sources.contains(&source_type) {
        return Err(format!("Unsupported source type: {}", source_type));
    }
    Ok(format!(
        "Source '{}' at {} appears reachable. Found objects: (simulated)",
        source_type, url
    ))
}

pub async fn migrate_run_import(
    source_type: &str,
    source_url: &str,
    target_url: &str,
    namespace: &str,
    mode: &str,
) -> Result<String, String> {
    let output = std::process::Command::new("primusdb")
        .args([
            "migrate",
            "import",
            source_type,
            source_url,
            target_url,
            "--namespace",
            namespace,
            "--mode",
            mode,
        ])
        .output()
        .map_err(|e| format!("Failed to run primusdb migrate: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub fn fetch_journalctl() -> String {
    let output = std::process::Command::new("journalctl")
        .args(["-u", "primusdb", "-n", "50", "--no-pager"])
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            format!("journalctl failed: {}", stderr)
        }
        Err(e) => format!("Could not run journalctl: {}", e),
    }
}
