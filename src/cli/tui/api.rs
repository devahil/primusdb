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

pub async fn explain_query(url: &str, query: &str) -> String {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("Client error: {}", e),
    };
    let body = serde_json::json!({ "query": query });
    match client
        .post(format!("{}/api/v1/query/explain", url))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let result: serde_json::Value = resp.json().await.unwrap_or_default();
            serde_json::to_string_pretty(&result).unwrap_or_default()
        }
        Err(e) => format!("Explain failed: {}", e),
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

pub async fn fetch_federation_status(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/federation/status", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_federation_clusters(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/federation/clusters", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let v: serde_json::Value = resp.json().await.ok()?;
        Some(
            v.get("data")
                .or_else(|| v.get("success").map(|_| &v))
                .cloned()
                .unwrap_or(v),
        )
    } else {
        None
    }
}

pub async fn fetch_federation_domains(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/federation/domains", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let v: serde_json::Value = resp.json().await.ok()?;
        Some(
            v.get("data")
                .or_else(|| v.get("success").map(|_| &v))
                .cloned()
                .unwrap_or(v),
        )
    } else {
        None
    }
}

pub async fn create_federation_cluster(
    url: &str,
    cluster_id: &str,
    seed: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
    let body = serde_json::json!({"id": cluster_id, "seed": seed});
    let resp = client
        .post(format!("{}/api/v1/federation/clusters", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Cluster '{}' added to federation", cluster_id))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to add cluster (HTTP {}): {}", status, text))
    }
}

pub async fn delete_federation_cluster(url: &str, cluster_id: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/federation/clusters/{}", url, cluster_id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Cluster '{}' removed from federation", cluster_id))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to remove cluster (HTTP {}): {}",
            status, text
        ))
    }
}

pub async fn create_federation_domain(
    url: &str,
    name: &str,
    cluster_ids: &[String],
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
    let body = serde_json::json!({"name": name, "clusters": cluster_ids});
    let resp = client
        .post(format!("{}/api/v1/federation/domains", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Domain '{}' created", name))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to create domain (HTTP {}): {}",
            status, text
        ))
    }
}

pub async fn delete_federation_domain(url: &str, name: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/federation/domains/{}", url, name))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Domain '{}' deleted", name))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to delete domain (HTTP {}): {}",
            status, text
        ))
    }
}

pub async fn fetch_governor_status(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/governor/status", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_governor_metrics(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/governor/metrics", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_governor_violations(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/governor/violations", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn fetch_governor_executions(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/governor/executions", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

pub async fn set_governor_policy(
    url: &str,
    name: &str,
    policy_json: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
    let body: serde_json::Value =
        serde_json::from_str(policy_json).map_err(|e| format!("Invalid JSON: {}", e))?;
    let resp = client
        .post(format!("{}/api/v1/governor/policies/{}", url, name))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Policy '{}' set", name))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to set policy (HTTP {}): {}", status, text))
    }
}

pub async fn delete_governor_policy(url: &str, name: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build failed: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/governor/policies/{}", url, name))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Policy '{}' deleted", name))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to delete policy (HTTP {}): {}",
            status, text
        ))
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
        Err(_) => {
            // Fallback: try info-schema for backward compatibility
            let fb = match client
                .get(format!("{}/api/v1/info-schema/relational/tables", url))
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            };
            if !fb.status().is_success() {
                return Vec::new();
            }
            let value: serde_json::Value = fb.json().await.unwrap_or_default();
            if let Some(data) = value.get("data").and_then(|d| d.as_array()) {
                return data
                    .iter()
                    .filter_map(|row| {
                        row.get("table_name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();
            }
            return Vec::new();
        }
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let value: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(data) = value.get("data").and_then(|d| d.as_array()) {
        data.iter()
            .filter_map(|row| {
                row.get("name")
                    .or_else(|| row.get("table_name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect()
    } else if let Some(arr) = value.as_array() {
        arr.iter()
            .filter_map(|v| {
                v.get("name")
                    .or_else(|| v.get("table_name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect()
    } else {
        Vec::new()
    }
}

pub async fn create_database(
    url: &str,
    name: &str,
    description: &str,
    engines: &[&str],
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "engines": engines,
    });
    let resp = client
        .post(format!("{}/api/v1/databases", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        let value: serde_json::Value = resp.json().await.unwrap_or_default();
        let msg = value
            .get("data")
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or(name);
        Ok(format!("Database '{}' created", msg))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to create database (HTTP {}): {}",
            status, text
        ))
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

// ── Config Studio API (v1.3.2-alpha) ────────────────────────────

pub async fn fetch_config_entries(url: &str) -> Option<serde_json::Value> {
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
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn set_config_entry(
    url: &str,
    key: &str,
    value: serde_json::Value,
    source: &str,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "key": key,
        "value": value,
        "source": source,
    });
    let resp = client
        .post(format!("{}/api/v1/config", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp.get("data").cloned().unwrap_or_default())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn delete_config_entry(url: &str, key: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/config", url))
        .json(&serde_json::json!({ "key": key }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn fetch_config_snapshots(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/config/snapshots", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn create_config_snapshot(
    url: &str,
    name: &str,
    description: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "name": name,
        "description": description,
    });
    let resp = client
        .post(format!("{}/api/v1/config/snapshots", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn restore_config_snapshot(url: &str, id: &str) -> Result<usize, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .post(format!("{}/api/v1/config/snapshots/{}/restore", url, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp
            .get("data")
            .and_then(|d| d.get("restored"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize)
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn delete_config_snapshot(url: &str, id: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/config/snapshots/{}", url, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn export_config_bundle(url: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .get(format!("{}/api/v1/config/export", url))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp.get("data").cloned().unwrap_or_default())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn import_config_bundle(url: &str, bundle: &serde_json::Value) -> Result<usize, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "bundle": bundle });
    let resp = client
        .post(format!("{}/api/v1/config/import", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp
            .get("data")
            .and_then(|d| d.get("imported"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize)
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

// ── Table Explorer API (v1.3.2-alpha) ─────────────────────────

pub async fn fetch_explorer_storage_types(url: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let resp = match client
        .get(format!("{}/api/v1/explorer/storage-types", url))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let api_resp: serde_json::Value = resp.json().await.unwrap_or_default();
    if let Some(data) = api_resp.get("data").and_then(|d| d.as_array()) {
        data.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        Vec::new()
    }
}

pub async fn fetch_explorer_tables(url: &str, storage_type: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!(
            "{}/api/v1/explorer/tables?storage_type={}",
            url, storage_type
        ))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn fetch_explorer_table_info(
    url: &str,
    storage_type: &str,
    table: &str,
) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!(
            "{}/api/v1/explorer/table/{}/{}",
            url, storage_type, table
        ))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn fetch_explorer_rows(
    url: &str,
    storage_type: &str,
    table: &str,
    limit: u64,
    offset: u64,
    filter: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let body = serde_json::json!({
        "limit": limit,
        "offset": offset,
        "filter": filter,
    });
    let resp = client
        .post(format!(
            "{}/api/v1/explorer/table/{}/{}/rows",
            url, storage_type, table
        ))
        .json(&body)
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

// ── RAG Workspace API (v1.3.2-alpha) ─────────────────────

pub async fn fetch_rag_collections(url: &str) -> Option<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!(
            "{}/api/v1/explorer/tables?storage_type=vector",
            url
        ))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        if let Some(data) = api_resp.get("data") {
            if let Some(tables) = data.get("tables").and_then(|t| t.as_array()) {
                let names: Vec<String> = tables
                    .iter()
                    .filter_map(|t| {
                        t.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();
                return Some(names);
            }
        }
    }
    None
}

pub async fn rag_search(
    url: &str,
    collection: &str,
    query: &str,
    limit: usize,
) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .ok()?;
    let body = serde_json::json!({
        "collection": collection,
        "query_text": query,
        "limit": limit,
    });
    let resp = client
        .post(format!("{}/api/v1/rag/search", url))
        .json(&body)
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

// ── Notebook API (v1.3.2-alpha) ──────────────────────────

pub async fn fetch_notebooks(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/notebooks", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn create_notebook(url: &str, name: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "name": name });
    let resp = client
        .post(format!("{}/api/v1/notebooks", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp.get("data").cloned().unwrap_or_default())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn fetch_notebook(url: &str, id: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/notebooks/{}", url, id))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn update_notebook(
    url: &str,
    id: &str,
    cells: Vec<serde_json::Value>,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "cells": cells });
    let resp = client
        .put(format!("{}/api/v1/notebooks/{}", url, id))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn delete_notebook(url: &str, id: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/notebooks/{}", url, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn execute_notebook_cell(
    url: &str,
    id: &str,
    cell_index: usize,
) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .ok()?;
    let body = serde_json::json!({ "cell_index": cell_index });
    let resp = client
        .post(format!("{}/api/v1/notebooks/{}/execute", url, id))
        .json(&body)
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

// ── Report Builder API (v1.3.2-alpha) ────────────────────────

pub async fn fetch_reports(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/reports", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn create_report(
    url: &str,
    name: &str,
    query: &str,
    description: &str,
    storage_type: &str,
    format: &str,
    table_name: &str,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "name": name,
        "query": query,
        "description": description,
        "storage_type": storage_type,
        "format": format,
        "table_name": table_name,
    });
    let resp = client
        .post(format!("{}/api/v1/reports", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp.get("data").cloned().unwrap_or_default())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn fetch_report(url: &str, id: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/reports/{}", url, id))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
    }
}

pub async fn delete_report(url: &str, id: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/reports/{}", url, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn update_report(
    url: &str,
    id: &str,
    name: &str,
    query: &str,
    description: &str,
    storage_type: &str,
    format: &str,
    table_name: &str,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "name": name,
        "query": query,
        "description": description,
        "storage_type": storage_type,
        "format": format,
        "table_name": table_name,
    });
    let resp = client
        .put(format!("{}/api/v1/reports/{}", url, id))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let api_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if api_resp
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        Ok(api_resp
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    } else {
        Err(api_resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string())
    }
}

pub async fn execute_report(
    url: &str,
    id: &str,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .ok()?;
    let body = serde_json::json!({
        "limit": limit,
        "offset": offset,
    });
    let resp = client
        .post(format!("{}/api/v1/reports/{}/execute", url, id))
        .json(&body)
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        let api_resp: serde_json::Value = resp.json().await.ok()?;
        api_resp.get("data").cloned()
    } else {
        None
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

// ── System Database Export / Backup Integration ────────────────────────

/// Export the system database configuration bundle as JSON.
/// This can be used for backup or migration of TUI/server settings.
pub async fn fetch_system_db_export(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/system/export", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

/// Import a system database configuration bundle.
/// Returns success message or error detail.
pub async fn fetch_system_db_import(
    url: &str,
    bundle: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/api/v1/system/import", url))
        .json(bundle)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if status.is_success() {
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(json
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Import successful")
            .to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Import failed (HTTP {}): {}", status, text))
    }
}

/// Create a namespace via the REST API.
pub async fn create_namespace(url: &str, path: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "path": path });
    let resp = client
        .post(format!("{}/api/v1/namespaces/{}", url, path))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Namespace '{}' created", path))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to create namespace (HTTP {}): {}",
            status, text
        ))
    }
}

/// Delete a namespace via the REST API.
pub async fn delete_namespace(url: &str, path: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/namespaces/{}", url, path))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Namespace '{}' deleted", path))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to delete namespace (HTTP {}): {}",
            status, text
        ))
    }
}

/// Create a table/collection via the REST API.
pub async fn create_table(
    url: &str,
    storage_type: &str,
    table: &str,
    schema: Option<&serde_json::Value>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let mut body = serde_json::json!({ "name": table });
    if let Some(s) = schema {
        body["schema"] = s.clone();
    }
    let resp = client
        .post(format!(
            "{}/api/v1/table/{}/{}/create",
            url, storage_type, table
        ))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Table '{}' created in {}", table, storage_type))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to create table (HTTP {}): {}",
            status, text
        ))
    }
}

/// Drop a table/collection via the REST API.
pub async fn drop_table(url: &str, storage_type: &str, table: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!(
            "{}/api/v1/table/{}/{}/drop",
            url, storage_type, table
        ))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Table '{}' dropped from {}", table, storage_type))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to drop table (HTTP {}): {}", status, text))
    }
}

/// Add a column to a table via POST /api/v1/ddl/:st/:table/column/add.
pub async fn add_column(
    url: &str,
    storage_type: &str,
    table: &str,
    column_def: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .post(format!(
            "{}/api/v1/ddl/{}/{}/column/add",
            url, storage_type, table
        ))
        .json(column_def)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Column added to '{}': {}", table, text))
    } else {
        Err(format!("Failed to add column (HTTP {}): {}", status, text))
    }
}

/// Drop a column from a table via DELETE /api/v1/ddl/:st/:table/column/:name.
pub async fn drop_column(
    url: &str,
    storage_type: &str,
    table: &str,
    column_name: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!(
            "{}/api/v1/ddl/{}/{}/column/{}",
            url, storage_type, table, column_name
        ))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Column '{}' dropped from '{}'", column_name, table))
    } else {
        Err(format!("Failed to drop column (HTTP {}): {}", status, text))
    }
}

/// Modify a column via PUT /api/v1/ddl/:st/:table/column.
pub async fn modify_column(
    url: &str,
    storage_type: &str,
    table: &str,
    column_def: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .put(format!(
            "{}/api/v1/ddl/{}/{}/column",
            url, storage_type, table
        ))
        .json(column_def)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Column modified in '{}': {}", table, text))
    } else {
        Err(format!(
            "Failed to modify column (HTTP {}): {}",
            status, text
        ))
    }
}

/// Add a constraint via POST /api/v1/ddl/:st/:table/constraint.
pub async fn add_constraint(
    url: &str,
    storage_type: &str,
    table: &str,
    constraint_def: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .post(format!(
            "{}/api/v1/ddl/{}/{}/constraint",
            url, storage_type, table
        ))
        .json(constraint_def)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Constraint added to '{}': {}", table, text))
    } else {
        Err(format!(
            "Failed to add constraint (HTTP {}): {}",
            status, text
        ))
    }
}

/// Drop a constraint via DELETE /api/v1/ddl/:st/:table/constraint/:name.
pub async fn drop_constraint(
    url: &str,
    storage_type: &str,
    table: &str,
    constraint_name: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!(
            "{}/api/v1/ddl/{}/{}/constraint/{}",
            url, storage_type, table, constraint_name
        ))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!(
            "Constraint '{}' dropped from '{}'",
            constraint_name, table
        ))
    } else {
        Err(format!(
            "Failed to drop constraint (HTTP {}): {}",
            status, text
        ))
    }
}

/// Rename a table via POST /api/v1/ddl/:st/:table/rename.
pub async fn rename_table(
    url: &str,
    storage_type: &str,
    table: &str,
    new_name: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "new_name": new_name });
    let resp = client
        .post(format!(
            "{}/api/v1/ddl/{}/{}/rename",
            url, storage_type, table
        ))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Table '{}' renamed to '{}'", table, new_name))
    } else {
        Err(format!(
            "Failed to rename table (HTTP {}): {}",
            status, text
        ))
    }
}

/// Insert a row into a table via the query API.
pub async fn insert_row(
    url: &str,
    storage_type: &str,
    table: &str,
    values: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "query": format!("INSERT INTO \"{}\" VALUES ?", table),
        "storage_type": storage_type,
        "params": values,
    });
    let resp = client
        .post(format!("{}/api/v1/query", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Row inserted into '{}'", table))
    } else {
        Err(format!("Insert failed (HTTP {}): {}", status, text))
    }
}

/// Delete a row from a table via the query API.
pub async fn delete_row(
    url: &str,
    storage_type: &str,
    table: &str,
    condition: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "query": format!("DELETE FROM \"{}\" WHERE {}", table, condition),
        "storage_type": storage_type,
    });
    let resp = client
        .post(format!("{}/api/v1/query", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Row deleted from '{}'", table))
    } else {
        Err(format!("Delete failed (HTTP {}): {}", status, text))
    }
}

/// Bulk export data from a table by iterating over query results.
/// Returns a JSON array of rows.
pub async fn export_table_data(
    url: &str,
    storage_type: &str,
    table: &str,
    limit: u64,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let query = format!("SELECT * FROM \"{}\" LIMIT {}", table, limit);
    let body = serde_json::json!({
        "query": query,
        "storage_type": storage_type,
    });
    let resp = client
        .post(format!("{}/api/v1/query", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        serde_json::from_str::<serde_json::Value>(&text).map_err(|e| format!("Parse failed: {}", e))
    } else {
        Err(format!("Export failed (HTTP {}): {}", status, text))
    }
}

/// Bulk import data into a table via the query API (INSERT statements).
pub async fn import_table_data(
    url: &str,
    storage_type: &str,
    table: &str,
    rows: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "query": format!("INSERT INTO \"{}\" VALUES ?", table),
        "storage_type": storage_type,
        "params": rows,
    });
    let resp = client
        .post(format!("{}/api/v1/query", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!("Import completed: {}", text))
    } else {
        Err(format!("Import failed (HTTP {}): {}", status, text))
    }
}

/// Create or replace a KV document via PUT.
pub async fn put_kv_document(
    url: &str,
    db: &str,
    doc_id: &str,
    json_data: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let data_value: serde_json::Value =
        serde_json::from_str(json_data).map_err(|e| format!("Invalid JSON: {}", e))?;
    let resp = client
        .put(format!("{}/api/v1/kv/{}/{}", url, db, doc_id))
        .json(&data_value)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!(
            "Document '{}' created/replaced in '{}': {}",
            doc_id, db, text
        ))
    } else {
        Err(format!(
            "Failed to create document (HTTP {}): {}",
            status, text
        ))
    }
}

/// Update (partial) a KV document via POST.
pub async fn update_kv_document(
    url: &str,
    db: &str,
    doc_id: &str,
    json_data: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let data_value: serde_json::Value =
        serde_json::from_str(json_data).map_err(|e| format!("Invalid JSON: {}", e))?;
    let resp = client
        .post(format!("{}/api/v1/kv/{}/{}", url, db, doc_id))
        .json(&data_value)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(format!(
            "Document '{}' updated in '{}': {}",
            doc_id, db, text
        ))
    } else {
        Err(format!(
            "Failed to update document (HTTP {}): {}",
            status, text
        ))
    }
}

/// Delete a KV document via the REST API.
pub async fn delete_kv_document(url: &str, db: &str, doc_id: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!("{}/api/v1/kv/{}/{}", url, db, doc_id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Document '{}' deleted from '{}'", doc_id, db))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to delete document (HTTP {}): {}",
            status, text
        ))
    }
}

/// Delete a user via the REST API (POST to auth endpoint).
pub async fn delete_user(url: &str, username: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .post(format!("{}/api/v1/auth/users/delete", url))
        .json(&serde_json::json!({ "username": username }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("User '{}' deleted", username))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to delete user (HTTP {}): {}", status, text))
    }
}

/// Delete a role via the REST API.
pub async fn delete_role(url: &str, role_name: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .post(format!("{}/api/v1/auth/roles/delete", url))
        .json(&serde_json::json!({ "name": role_name }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Role '{}' deleted", role_name))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to delete role (HTTP {}): {}", status, text))
    }
}

/// Fetch RBAC permissions via the REST API.
pub async fn fetch_permissions(url: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{}/api/v1/auth/permissions", url))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json().await.ok()
    } else {
        None
    }
}

/// Assign a role to a user via the REST API.
pub async fn assign_user_role(
    url: &str,
    username: &str,
    role_name: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "role": role_name });
    let resp = client
        .post(format!("{}/api/v1/auth/users/{}/roles", url, username))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Role '{}' assigned to '{}'", role_name, username))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to assign role (HTTP {}): {}", status, text))
    }
}

/// Remove a role from a user via the REST API.
pub async fn remove_user_role(
    url: &str,
    username: &str,
    role_name: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .delete(format!(
            "{}/api/v1/auth/users/{}/roles/{}",
            url, username, role_name
        ))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Role '{}' removed from '{}'", role_name, username))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to remove role (HTTP {}): {}", status, text))
    }
}

/// Save TUI configuration to the server's system database.
pub async fn fetch_save_tui_config(
    url: &str,
    config: &serde_json::Value,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/api/v1/system/tui-config", url))
        .json(config)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if status.is_success() {
        Ok("TUI config saved".to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!(
            "Failed to save TUI config (HTTP {}): {}",
            status, text
        ))
    }
}

/// Create a user via the REST API.
pub async fn create_user(url: &str, username: &str, password: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({
        "username": username,
        "password": password,
    });
    let resp = client
        .post(format!("{}/api/v1/auth/register", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("User '{}' created", username))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to create user (HTTP {}): {}", status, text))
    }
}

/// Create a role via the REST API.
pub async fn create_role(url: &str, name: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let body = serde_json::json!({ "name": name });
    let resp = client
        .post(format!("{}/api/v1/auth/roles", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    if status.is_success() {
        Ok(format!("Role '{}' created", name))
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Failed to create role (HTTP {}): {}", status, text))
    }
}

pub async fn analyze_table(url: &str, storage_type: &str, table: &str) -> String {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("Client error: {}", e),
    };
    let body = serde_json::json!({
        "query": format!("ANALYZE TABLE \"{}\"", table),
        "storage_type": storage_type,
    });
    match client
        .post(format!("{}/api/v1/query", url))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let result: serde_json::Value = resp.json().await.unwrap_or_default();
            serde_json::to_string_pretty(&result).unwrap_or_default()
        }
        Err(e) => format!("Analyze failed: {}", e),
    }
}

pub async fn cluster_start(url: &str) -> String {
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/v1/cluster/start", url))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                "Cluster started".to_string()
            } else {
                text
            }
        }
        Err(e) => format!("Cluster start failed: {}", e),
    }
}

pub async fn cluster_stop(url: &str) -> String {
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/v1/cluster/stop", url))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                "Cluster stopped".to_string()
            } else {
                text
            }
        }
        Err(e) => format!("Cluster stop failed: {}", e),
    }
}

pub async fn cluster_restart(url: &str) -> String {
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/v1/cluster/restart", url))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                "Cluster restarted".to_string()
            } else {
                text
            }
        }
        Err(e) => format!("Cluster restart failed: {}", e),
    }
}

pub async fn cluster_join(url: &str, target: &str) -> String {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "target": target });
    match client
        .post(format!("{}/api/v1/cluster/join", url))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                format!("Joined cluster at {}", target)
            } else {
                text
            }
        }
        Err(e) => format!("Cluster join failed: {}", e),
    }
}

pub async fn cluster_leave(url: &str) -> String {
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/v1/cluster/leave", url))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                "Left the cluster".to_string()
            } else {
                text
            }
        }
        Err(e) => format!("Cluster leave failed: {}", e),
    }
}

pub async fn cluster_remove_node(url: &str) -> String {
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/v1/cluster/node/remove", url))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                "Node removed from cluster".to_string()
            } else {
                text
            }
        }
        Err(e) => format!("Node removal failed: {}", e),
    }
}

pub async fn cluster_maintenance(url: &str) -> String {
    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/v1/cluster/maintenance", url))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                "Maintenance mode toggled".to_string()
            } else {
                text
            }
        }
        Err(e) => format!("Maintenance toggle failed: {}", e),
    }
}

pub async fn delete_rag_collection(url: &str, name: &str) -> Result<String, String> {
    drop_table(url, "vector", name).await
}

pub fn delete_backup_local(id: &str) -> Result<String, String> {
    let backup_dir = std::path::Path::new("backups");
    let mut removed = 0;
    if let Ok(entries) = std::fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(id) {
                if entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                    std::fs::remove_dir_all(entry.path())
                        .map_err(|e| format!("Failed to remove directory: {}", e))?;
                } else {
                    std::fs::remove_file(entry.path())
                        .map_err(|e| format!("Failed to remove file: {}", e))?;
                }
                removed += 1;
            }
        }
    }
    if removed > 0 {
        Ok(format!(
            "Deleted {} backup file(s) matching '{}'",
            removed, id
        ))
    } else {
        Err(format!("No backup files found matching '{}'", id))
    }
}

pub fn delete_file_local(path: &str) -> Result<String, String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("File '{}' does not exist", path));
    }
    if p.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| format!("Failed to remove directory: {}", e))?;
    } else {
        std::fs::remove_file(p).map_err(|e| format!("Failed to remove file: {}", e))?;
    }
    Ok(format!("Deleted '{}'", path))
}
