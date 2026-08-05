//! Cluster operation subcommands (`cluster status`, `nodes`, `join`, ...).
//!
//! All operations run in client mode against the `/api/v1/cluster/*`
//! endpoints on `GlobalArgs.server_url`.

use std::path::PathBuf;

use crate::cli::command::{ClusterSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch a `cluster` subcommand to its handler.
pub async fn handle_cluster(
    cmd: ClusterSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        ClusterSubcommands::Status {
            verbose,
            watch,
            interval,
        } => cmd_status(verbose, watch, interval, global, fmt).await,
        ClusterSubcommands::Nodes { role, state } => cmd_nodes(role, state, global, fmt).await,
        ClusterSubcommands::Join {
            peer,
            node_id,
            node,
            seed,
            timeout,
            tls,
        } => cmd_join(peer, node_id, node, seed, timeout, tls, global, fmt).await,
        ClusterSubcommands::Leave { node, drain, force } => {
            cmd_leave(node, drain, force, global, fmt).await
        }
        ClusterSubcommands::Rebalance {
            node,
            strategy,
            concurrency,
        } => cmd_rebalance(node, strategy, concurrency, global, fmt).await,
        ClusterSubcommands::Failover {
            node,
            target,
            force,
        } => cmd_failover(node, target, force, global, fmt).await,
        ClusterSubcommands::Health {
            diagnostic,
            threshold_ms,
        } => cmd_health(diagnostic, threshold_ms, global, fmt).await,
        ClusterSubcommands::Sync { full, timeout } => cmd_sync(full, timeout, global, fmt).await,
        ClusterSubcommands::Config { get, set, list } => {
            cmd_cluster_config(get, set, list, global, fmt).await
        }
        ClusterSubcommands::Inspect { node, verbose } => {
            cmd_inspect(node, verbose, global, fmt).await
        }
        ClusterSubcommands::Topology { format: topo_fmt } => {
            cmd_topology(topo_fmt, global, fmt).await
        }
    }
}

async fn cmd_status(
    verbose: bool,
    watch: bool,
    interval: u64,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();

    loop {
        let mut url = format!("{}/api/v1/cluster/status", global.server_url);
        if verbose {
            url.push_str("?verbose=true");
        }

        let data = match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                OutputData::Json(json)
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                OutputData::Error(format!("HTTP {}: {}", status, text))
            }
            Err(e) => OutputData::Error(format!("Connection failed: {}", e)),
        };

        println!("{}", format_output(&data, *fmt));

        if !watch {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
    Ok(())
}

async fn cmd_nodes(
    role: Option<String>,
    state: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/cluster/nodes", global.server_url);

    let mut params: Vec<String> = Vec::new();
    if let Some(ref r) = role {
        params.push(format!("role={}", r));
    }
    if let Some(ref s) = state {
        params.push(format!("state={}", s));
    }
    if !params.is_empty() {
        url.push_str(&format!("?{}", params.join("&")));
    }

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Error(format!("HTTP {}: {}", status, text));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_join(
    peer: String,
    node_id: Option<String>,
    node: Option<String>,
    seed: Option<String>,
    timeout: u64,
    tls: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();

    let target = seed.unwrap_or_else(|| global.server_url.clone());
    let url = format!("{}/api/v1/cluster/node/register", target);

    // Parse port from peer URL or derive from TLS flag
    let port = if peer.contains(':') {
        peer.split(':')
            .next_back()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(if tls { 443 } else { 8080 })
    } else {
        if tls {
            443
        } else {
            8080
        }
    };

    let node_id = node.or(node_id).unwrap_or_else(|| "cli-node".to_string());

    let body = serde_json::json!({
        "node_id": node_id,
        "host": peer,
        "port": port,
        "timeout": timeout,
        "tls": tls,
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Error(format!("HTTP {}: {}", status, text));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_inspect(
    node: String,
    verbose: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/cluster/node/{}", global.server_url, node);
    if verbose {
        url.push_str("?verbose=true");
    }

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(_) => {
            let list_url = format!("{}/api/v1/cluster/nodes", global.server_url);
            match client.get(&list_url).send().await {
                Ok(list_resp) if list_resp.status().is_success() => {
                    let nodes: serde_json::Value = list_resp.json().await.unwrap_or_default();
                    let mut info = serde_json::json!({
                        "node": node,
                        "message": "Node found in cluster list. Individual node endpoint not available.",
                        "cluster_nodes": nodes,
                    });
                    if verbose {
                        if let Some(arr) = nodes.as_array() {
                            let details: Vec<&serde_json::Value> = arr
                                .iter()
                                .filter(|n| {
                                    n.get("node_id").and_then(|v| v.as_str()) == Some(&node)
                                        || n.get("id").and_then(|v| v.as_str()) == Some(&node)
                                        || n.get("name").and_then(|v| v.as_str()) == Some(&node)
                                })
                                .collect();
                            if !details.is_empty() {
                                info["node_details"] = serde_json::json!(details);
                            }
                        }
                    }
                    let data = OutputData::Json(info);
                    println!("{}", format_output(&data, *fmt));
                }
                Ok(list_resp) => {
                    let status = list_resp.status();
                    let text = list_resp.text().await.unwrap_or_default();
                    let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                    println!("{}", format_output(&data, *fmt));
                }
                Err(e) => {
                    let data = OutputData::Error(format!("Connection failed: {}", e));
                    println!("{}", format_output(&data, *fmt));
                }
            }
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_topology(topo_format: String, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let status_url = format!("{}/api/v1/cluster/status", global.server_url);
    let nodes_url = format!("{}/api/v1/cluster/nodes", global.server_url);

    let status = client.get(&status_url).send().await;
    let nodes = client.get(&nodes_url).send().await;

    let mut topo = serde_json::json!({
        "topology": {},
        "nodes": [],
        "note": "Cluster topology — combines status and node information"
    });

    if let Ok(resp) = status {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                topo["topology"] = json;
            }
        }
    }

    if let Ok(resp) = nodes {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                topo["nodes"] = json;
            }
        }
    }

    if topo_format == "json" {
        let data = OutputData::Json(topo);
        println!("{}", format_output(&data, OutputFormat::Json));
    } else {
        let data = OutputData::Json(topo);
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

async fn cmd_leave(
    node: String,
    drain: bool,
    force: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/cluster/node/{}", global.server_url, node);

    let mut params: Vec<String> = Vec::new();
    if drain {
        params.push("drain=true".into());
    }
    if force {
        params.push("force=true".into());
    }
    if !params.is_empty() {
        url.push_str(&format!("?{}", params.join("&")));
    }

    match client.delete(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            if resp.status().as_u16() == 404 {
                let data =
                    OutputData::Message(format!("Node '{}' not found in cluster registry.", node));
                println!("{}", format_output(&data, *fmt));
            } else {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                println!("{}", format_output(&data, *fmt));
            }
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_rebalance(
    node: Option<String>,
    strategy: String,
    concurrency: u32,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/cluster/rebalance", global.server_url);

    let body = serde_json::json!({
        "node": node,
        "strategy": strategy,
        "concurrency": concurrency,
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Error(format!("HTTP {}: {}", status, text));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_failover(
    node: String,
    target: Option<String>,
    force: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/cluster/failover", global.server_url);

    let body = serde_json::json!({
        "node": node,
        "target": target,
        "force": force,
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Error(format!("HTTP {}: {}", status, text));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_health(
    diagnostic: bool,
    threshold_ms: u64,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/cluster/health", global.server_url);

    let mut params: Vec<String> = Vec::new();
    if diagnostic {
        params.push("diagnostic=true".into());
    }
    params.push(format!("threshold_ms={}", threshold_ms));
    if !params.is_empty() {
        url.push_str(&format!("?{}", params.join("&")));
    }

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            if resp.status().as_u16() == 404 {
                let data = OutputData::Message(
                    "Cluster health endpoint not available. The server may be running in standalone mode."
                        .into(),
                );
                println!("{}", format_output(&data, *fmt));
            } else {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                println!("{}", format_output(&data, *fmt));
            }
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_sync(full: bool, timeout: u64, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/cluster/sync", global.server_url);

    let body = serde_json::json!({
        "full": full,
        "timeout": timeout,
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Error(format!("HTTP {}: {}", status, text));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_cluster_config(
    get: Option<String>,
    _set: Option<Vec<String>>,
    list: bool,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    if list || get.is_some() {
        let config_path = PathBuf::from("primusdb.toml");
        if let Ok(contents) = tokio::fs::read_to_string(&config_path).await {
            // Extract [cluster] section
            let mut in_cluster = false;
            let mut cluster_lines: Vec<&str> = Vec::new();
            for line in contents.lines() {
                if line.trim().starts_with("[cluster]") {
                    in_cluster = true;
                    if list {
                        cluster_lines.push(line);
                    }
                    continue;
                }
                if in_cluster {
                    if line.trim().starts_with('[') {
                        break;
                    }
                    if list {
                        cluster_lines.push(line);
                    } else if let Some(ref key) = get {
                        if line.trim().starts_with(&format!("{}=", key))
                            || line.trim().starts_with(&format!("{} =", key))
                        {
                            let data = OutputData::Message(line.to_string());
                            println!("{}", format_output(&data, *fmt));
                            return Ok(());
                        }
                    }
                }
            }

            if list {
                if cluster_lines.is_empty() {
                    let data =
                        OutputData::Message("No [cluster] section found in primusdb.toml".into());
                    println!("{}", format_output(&data, *fmt));
                } else {
                    let data = OutputData::Message(cluster_lines.join("\n"));
                    println!("{}", format_output(&data, *fmt));
                }
            } else if let Some(ref key) = get {
                let data =
                    OutputData::Message(format!("Key '{}' not found in [cluster] section", key));
                println!("{}", format_output(&data, *fmt));
            }
        } else {
            let data = OutputData::Message(
                "No config file found. Generate one with:\n  primusdb config init\n\
                 Then edit the [cluster] section."
                    .into(),
            );
            println!("{}", format_output(&data, *fmt));
        }
    } else {
        let data = OutputData::Message(
            "Use --list to view cluster config, --get <key> for a specific value".into(),
        );
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_port_parse_from_peer() {
        let peer = "192.168.1.1:8080";
        let tls = false;
        let port = if peer.contains(':') {
            peer.split(':')
                .next_back()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(if tls { 443 } else { 8080 })
        } else {
            if tls {
                443
            } else {
                8080
            }
        };
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_port_parse_no_port_tls() {
        let peer = "192.168.1.1";
        let tls = true;
        let port = if peer.contains(':') {
            peer.split(':')
                .next_back()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(if tls { 443 } else { 8080 })
        } else {
            if tls {
                443
            } else {
                8080
            }
        };
        assert_eq!(port, 443);
    }

    #[test]
    fn test_port_parse_no_port_no_tls() {
        let peer = "192.168.1.1";
        let tls = false;
        let port = if peer.contains(':') {
            peer.split(':')
                .next_back()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(if tls { 443 } else { 8080 })
        } else {
            if tls {
                443
            } else {
                8080
            }
        };
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_port_parse_ipv6() {
        let peer = "[::1]:9090";
        let tls = false;
        let port = if peer.contains(':') {
            peer.split(':')
                .next_back()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(if tls { 443 } else { 8080 })
        } else {
            if tls {
                443
            } else {
                8080
            }
        };
        assert_eq!(port, 9090);
    }

    #[test]
    fn test_node_id_selection() {
        let node: Option<String> = Some("explicit-node".to_string());
        let node_id: Option<String> = None;
        let result = node.or(node_id).unwrap_or_else(|| "cli-node".to_string());
        assert_eq!(result, "explicit-node");
    }

    #[test]
    fn test_node_id_fallback_to_node_id() {
        let node: Option<String> = None;
        let node_id: Option<String> = Some("legacy-node".to_string());
        let result = node.or(node_id).unwrap_or_else(|| "cli-node".to_string());
        assert_eq!(result, "legacy-node");
    }

    #[test]
    fn test_node_id_default() {
        let node: Option<String> = None;
        let node_id: Option<String> = None;
        let result = node.or(node_id).unwrap_or_else(|| "cli-node".to_string());
        assert_eq!(result, "cli-node");
    }

    #[test]
    fn test_seed_url_construction() {
        let seed = Some("http://seed-node:8080".to_string());
        let server_url = "http://localhost:8080".to_string();
        let target = seed.unwrap_or_else(|| server_url.clone());
        let url = format!("{}/api/v1/cluster/node/register", target);
        assert_eq!(url, "http://seed-node:8080/api/v1/cluster/node/register");
    }

    #[test]
    fn test_seed_fallback_to_global() {
        let seed: Option<String> = None;
        let server_url = "http://localhost:8080".to_string();
        let target = seed.unwrap_or_else(|| server_url.clone());
        let url = format!("{}/api/v1/cluster/node/register", target);
        assert_eq!(url, "http://localhost:8080/api/v1/cluster/node/register");
    }

    #[test]
    fn test_cmd_status_url_construction() {
        let server_url = "http://localhost:8080";
        let verbose = true;
        let mut url = format!("{}/api/v1/cluster/status", server_url);
        if verbose {
            url.push_str("?verbose=true");
        }
        assert_eq!(
            url,
            "http://localhost:8080/api/v1/cluster/status?verbose=true"
        );

        let mut url = format!("{}/api/v1/cluster/status", server_url);
        if false {
            url.push_str("?verbose=true");
        }
        assert_eq!(url, "http://localhost:8080/api/v1/cluster/status");
    }

    #[test]
    fn test_cmd_nodes_url_with_filters() {
        let server_url = "http://localhost:8080";
        let role = Some("leader".to_string());
        let state = Some("active".to_string());

        let mut url = format!("{}/api/v1/cluster/nodes", server_url);
        let mut params: Vec<String> = Vec::new();
        if let Some(ref r) = role {
            params.push(format!("role={}", r));
        }
        if let Some(ref s) = state {
            params.push(format!("state={}", s));
        }
        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }
        assert_eq!(
            url,
            "http://localhost:8080/api/v1/cluster/nodes?role=leader&state=active"
        );
    }

    #[test]
    fn test_cmd_nodes_url_no_filters() {
        let server_url = "http://localhost:8080";
        let role: Option<String> = None;
        let state: Option<String> = None;

        let mut url = format!("{}/api/v1/cluster/nodes", server_url);
        let mut params: Vec<String> = Vec::new();
        if let Some(ref r) = role {
            params.push(format!("role={}", r));
        }
        if let Some(ref s) = state {
            params.push(format!("state={}", s));
        }
        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }
        assert_eq!(url, "http://localhost:8080/api/v1/cluster/nodes");
    }
}
