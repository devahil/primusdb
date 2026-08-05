//! Server lifecycle subcommands (`server start`, `stop`, `restart`, ...).
//!
//! `server start` is the **server mode** entry point: it builds a
//! [`crate::PrimusDB`] instance and an [`crate::api::APIServer`] in-process.
//! The remaining subcommands operate on a running server via the local port
//! or process list.

use std::path::PathBuf;

use crate::cli::command::ServerSubcommands;
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Locate the PID of the process listening on `port` (via `lsof`).
fn find_process_on_port(port: u16) -> Option<u32> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("lsof -ti tcp:{} 2>/dev/null", port))
        .output()
        .ok()?;
    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    pid_str.parse::<u32>().ok()
}

fn kill_process(pid: u32, force: bool) -> bool {
    let signal = if force { "-9" } else { "-15" };
    std::process::Command::new("kill")
        .arg(signal)
        .arg(pid.to_string())
        .status()
        .ok()
        .is_some_and(|s| s.success())
}

/// Dispatch a `server` subcommand to its handler.
pub async fn handle_server(cmd: ServerSubcommands, fmt: &OutputFormat) -> Result<()> {
    match cmd {
        ServerSubcommands::Start {
            config,
            bind,
            data_dir,
            daemon,
            log_level,
            federation_id,
            cluster_id,
            region,
            federation_discovery,
            tls_enabled,
            tls_cert,
            tls_key,
        } => {
            cmd_start(
                config,
                bind,
                data_dir,
                daemon,
                log_level,
                federation_id,
                cluster_id,
                region,
                federation_discovery,
                tls_enabled,
                tls_cert,
                tls_key,
                fmt,
            )
            .await
        }
        ServerSubcommands::Stop { timeout, force } => cmd_stop(timeout, force, fmt).await,
        ServerSubcommands::Restart { config, timeout } => cmd_restart(config, timeout, fmt).await,
        ServerSubcommands::Status { verbose } => cmd_status(verbose, fmt).await,
        ServerSubcommands::Health { deep } => cmd_health(deep, fmt).await,
        ServerSubcommands::Config {
            get,
            set,
            list,
            file,
        } => cmd_config(get, set, list, file, fmt).await,
    }
}

// CLI command handler; each arg maps to an independent CLI flag.
#[allow(clippy::too_many_arguments)]
async fn cmd_start(
    config: Option<PathBuf>,
    bind: Option<String>,
    data_dir: Option<PathBuf>,
    _daemon: bool,
    log_level: String,
    federation_id: String,
    cluster_id: Option<String>,
    region: Option<String>,
    federation_discovery: Vec<String>,
    tls_enabled: bool,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    _fmt: &OutputFormat,
) -> Result<()> {
    let bind = bind.unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let parts: Vec<&str> = bind.split(':').collect();
    let host = parts.first().unwrap_or(&"127.0.0.1").to_string();
    let port: u16 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(8080);

    std::env::set_var("RUST_LOG", &log_level);
    let _ = tracing_subscriber::fmt::try_init();

    let data_dir = data_dir.map(|d| d.to_string_lossy().to_string());

    let mut primus_config = crate::PrimusDBConfig::default();
    primus_config.network.bind_address = host.clone();
    primus_config.network.port = port;
    primus_config.network.tls_enabled = tls_enabled;
    if let Some(ref cert) = tls_cert {
        primus_config.network.tls_cert_path = cert.clone();
    }
    if let Some(ref key) = tls_key {
        primus_config.network.tls_key_path = key.clone();
    }

    // Federation config
    let federation = if !federation_discovery.is_empty() {
        let net = &primus_config.network;
        Some(crate::cluster::FederationConfig {
            federation_id,
            cluster_id: cluster_id.unwrap_or_else(|| host.clone()),
            region: region.clone(),
            announce_interval_ms: 10_000,
            heartbeat_interval_ms: 5_000,
            heartbeat_timeout_ms: 3_000,
            suspect_timeout_ms: 30_000,
            max_clusters: 64,
            enable_cross_cluster_replication: true,
            enable_federated_namespaces: true,
            tls_cert_path: net.tls_cert_path.clone(),
            tls_key_path: net.tls_key_path.clone(),
            tls_ca_path: net.tls_ca_path.clone(),
            mtls_enabled: net.mtls_enabled,
        })
    } else {
        None
    };
    primus_config.federation = federation;
    primus_config.cluster.enabled = !federation_discovery.is_empty();

    if let Some(ref dir) = data_dir {
        primus_config.storage.data_dir = dir.clone();
    }
    if let Some(ref _cfg) = config {
        // Future: load config.toml
    }

    let network_config = primus_config.network.clone();
    let federation_enabled = primus_config.federation.is_some();
    let db = std::sync::Arc::new(crate::PrimusDB::new(primus_config)?);

    // Federation background tasks (announce + heartbeat loops) when enabled.
    if federation_enabled {
        db.start_federation().await;
    }

    let auth_config = crate::auth::AuthConfig {
        require_auth: true,
        min_password_length: 8,
        password_expiry_days: 90,
        max_login_attempts: 5,
        lockout_duration_minutes: 30,
        token_expiry_hours: 8760,
        session_timeout_minutes: 60,
        mfa_required_for_roles: vec!["admin".to_string()],
    };
    let auth_service = std::sync::Arc::new(crate::auth::AuthService::new(auth_config)?);

    let api_server =
        crate::api::APIServer::with_network_config(db, auth_service, None, network_config);

    println!("Starting PrimusDB server on {}...", bind);
    api_server.run(&bind).await?;

    Ok(())
}

async fn cmd_stop(timeout: u64, force: bool, fmt: &OutputFormat) -> Result<()> {
    let default_port = 8080;
    if let Some(pid) = find_process_on_port(default_port) {
        if kill_process(pid, force) {
            let action = if force { "killed" } else { "stopped" };
            let data = OutputData::Message(format!(
                "Server (PID: {}) {} on port {}",
                pid, action, default_port
            ));
            println!("{}", format_output(&data, *fmt));
        } else {
            let data = OutputData::Error(format!(
                "Failed to {} process {} on port {}",
                if force { "kill" } else { "stop" },
                pid,
                default_port
            ));
            println!("{}", format_output(&data, *fmt));
        }
    } else {
        let timeout_s = if timeout > 0 { timeout } else { 5 };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_s);

        loop {
            if let Some(pid) = find_process_on_port(default_port) {
                if kill_process(pid, force) {
                    let action = if force { "killed" } else { "stopped" };
                    let data = OutputData::Message(format!(
                        "Server (PID: {}) {} on port {}",
                        pid, action, default_port
                    ));
                    println!("{}", format_output(&data, *fmt));
                    break;
                }
            }
            if std::time::Instant::now() >= deadline {
                let data = OutputData::Message(format!(
                    "No server found on port {} after {}s timeout",
                    default_port, timeout_s
                ));
                println!("{}", format_output(&data, *fmt));
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    Ok(())
}

async fn cmd_restart(config: Option<PathBuf>, timeout: u64, fmt: &OutputFormat) -> Result<()> {
    let data = OutputData::Message(format!(
        "Restart initiated with {}s timeout. Use:\n  \
         primusdb server stop\n  \
         primusdb server start{}",
        timeout,
        config
            .map(|p| format!(" --config {}", p.display()))
            .unwrap_or_default()
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn cmd_status(verbose: bool, fmt: &OutputFormat) -> Result<()> {
    let port = 8080;
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let Some(pid) = find_process_on_port(port) {
        rows.push(vec!["Status".into(), "Running".into()]);
        rows.push(vec!["PID".into(), pid.to_string()]);

        // Try to get version from the running server
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

        match client
            .get(format!("http://127.0.0.1:{}/health", port))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(ver) = json.get("version").and_then(|v| v.as_str()) {
                        rows.push(vec!["Version".into(), ver.into()]);
                    }
                }
            }
            _ => {
                rows.push(vec!["Version".into(), env!("CARGO_PKG_VERSION").into()]);
            }
        }

        if verbose {
            rows.push(vec!["Build".into(), env!("CARGO_PKG_NAME").into()]);
            rows.push(vec!["License".into(), env!("CARGO_PKG_LICENSE").into()]);
            rows.push(vec!["Port".into(), port.to_string()]);
        }
    } else {
        rows.push(vec!["Status".into(), "Not running".into()]);
    }

    let data = OutputData::Table {
        headers: vec!["Key".into(), "Value".into()],
        rows,
    };
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn cmd_health(deep: bool, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = if deep {
        "http://127.0.0.1:8080/health?deep=true"
    } else {
        "http://127.0.0.1:8080/health"
    };

    match client.get(url).send().await {
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

async fn cmd_config(
    get: Option<String>,
    _set: Option<Vec<String>>,
    list: bool,
    file: Option<PathBuf>,
    fmt: &OutputFormat,
) -> Result<()> {
    let config_path = file.unwrap_or_else(|| PathBuf::from("primusdb.toml"));

    if list || get.is_some() {
        match tokio::fs::read_to_string(&config_path).await {
            Ok(contents) => {
                if let Some(key) = get {
                    // Try to extract a specific key
                    for line in contents.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with(&format!("{}=", key))
                            || trimmed.starts_with(&format!("{} =", key))
                        {
                            let data = OutputData::Message(line.to_string());
                            println!("{}", format_output(&data, *fmt));
                            return Ok(());
                        }
                    }
                    let data = OutputData::Message(format!(
                        "Key '{}' not found in {}",
                        key,
                        config_path.display()
                    ));
                    println!("{}", format_output(&data, *fmt));
                } else {
                    let data = OutputData::Message(contents);
                    println!("{}", format_output(&data, *fmt));
                }
            }
            Err(_) => {
                let data = OutputData::Message(format!(
                    "No config file found at {}. Generate one with:\n  \
                     primusdb config init",
                    config_path.display()
                ));
                println!("{}", format_output(&data, *fmt));
            }
        }
    } else {
        let data = OutputData::Message(
            "Use --list to view config, --get <key> for a specific value, or --set <key=value> to modify"
                .into(),
        );
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}
