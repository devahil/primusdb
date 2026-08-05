//! Protocol-layer subcommands (`protocol health`, `status`, `peers`,
//! `metrics`).
//!
//! All operations run in client mode against the `/protocol/*` endpoints on
//! `GlobalArgs.server_url`.

use crate::cli::command::{GlobalArgs, ProtocolSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch a `protocol` subcommand to its handler.
pub async fn handle_protocol(
    cmd: ProtocolSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        ProtocolSubcommands::Health { module } => cmd_health(module, global, fmt).await,
        ProtocolSubcommands::Status {
            versions,
            connections,
        } => cmd_status(versions, connections, global, fmt).await,
        ProtocolSubcommands::Peers { state, verbose } => {
            cmd_peers(state, verbose, global, fmt).await
        }
        ProtocolSubcommands::Metrics { protocol, raw } => {
            cmd_metrics(protocol, raw, global, fmt).await
        }
    }
}

async fn cmd_health(
    _module: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/protocol/health", global.server_url);

    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let data = OutputData::Json(json);
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

async fn cmd_status(
    _versions: bool,
    _connections: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/protocol/status", global.server_url);

    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let data = OutputData::Json(json);
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

async fn cmd_peers(
    _state: Option<String>,
    _verbose: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/protocol/peers", global.server_url);

    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let data = OutputData::Json(json);
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

async fn cmd_metrics(
    _protocol: String,
    _raw: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/protocol/metrics", global.server_url);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Message(text);
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
