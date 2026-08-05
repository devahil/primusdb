//! Storage engine subcommands (`engine list`, `status`, `inspect`,
//! `metrics`, `add`, `remove`, `upgrade`).
//!
//! Reads are client-mode HTTP calls; `add`/`remove`/`upgrade` offer REST
//! hot operations and fall back to configuration guidance.

use crate::cli::command::{EngineSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch an `engine` subcommand to its handler.
pub async fn handle_engine(
    cmd: EngineSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        EngineSubcommands::List { verbose } => cmd_list(verbose, global, fmt).await,
        EngineSubcommands::Status { name } => cmd_status(name, global, fmt).await,
        EngineSubcommands::Inspect {
            name,
            component,
            raw,
        } => cmd_inspect(name, component, raw, global, fmt).await,
        EngineSubcommands::Metrics { name, filter } => cmd_metrics(name, filter, global, fmt).await,
        EngineSubcommands::Add {
            engine_type,
            server,
            hot,
        } => cmd_add(engine_type, server, hot, fmt).await,
        EngineSubcommands::Remove {
            engine_type,
            server,
            force,
        } => cmd_remove(engine_type, server, force, fmt).await,
        EngineSubcommands::Upgrade {
            engine_type,
            server,
        } => cmd_upgrade(engine_type, server, fmt).await,
    }
}

async fn cmd_list(verbose: bool, _global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let engines = vec![
        ("columnar".into(), "Column-oriented analytics engine".into()),
        ("vector".into(), "Vector similarity search".into()),
        ("document".into(), "JSON document store".into()),
        ("relational".into(), "Relational SQL engine".into()),
        (
            "timeseries".into(),
            "Time series engine for IoT, metrics, logs".into(),
        ),
    ];

    if verbose {
        let rows: Vec<Vec<String>> = engines.into_iter().map(|(n, d)| vec![n, d]).collect();
        let data = OutputData::Table {
            headers: vec!["Engine".into(), "Description".into()],
            rows,
        };
        println!("{}", format_output(&data, *fmt));
    } else {
        let data = OutputData::List(engines.into_iter().map(|(n, _)| n).collect());
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

async fn cmd_status(name: String, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/table/{}/_default/info", global.server_url, name);

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

async fn cmd_inspect(
    name: String,
    component: Option<String>,
    raw: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let tail = component.as_deref().unwrap_or("");
    let url = format!(
        "{}/api/v1/engine/{}/inspect{}",
        global.server_url, name, tail
    );

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            if raw {
                let data = OutputData::Json(json);
                println!("{}", format_output(&data, *fmt));
            } else {
                let rows: Vec<Vec<String>> = json
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| {
                                vec![
                                    k.clone(),
                                    serde_json::to_string_pretty(v).unwrap_or_default(),
                                ]
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let data = OutputData::Table {
                    headers: vec!["Key".into(), "Value".into()],
                    rows,
                };
                println!("{}", format_output(&data, *fmt));
            }
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

async fn cmd_metrics(
    name: String,
    filter: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/engine/{}/metrics", global.server_url, name);
    if let Some(ref f) = filter {
        url.push_str(&format!("?filter={}", f));
    }

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            // Metrics are typically Prometheus text format
            let data = OutputData::Message(body);
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

async fn cmd_add(engine_type: String, server: String, hot: bool, fmt: &OutputFormat) -> Result<()> {
    if !hot {
        let msg = format!(
            "To add engine '{}':\n\
             1. Add it to your primusdb.toml under [storage.engines]\n\
             2. Restart the server: primusdb server restart --server {}\n\
             3. Verify: primusdb engine list",
            engine_type, server,
        );
        let data = OutputData::Message(msg);
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }
    // Hot-add via REST API
    print!("Hot-adding engine '{}' to {}... ", engine_type, server);
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/engine/{}/add", server, engine_type);
    match client.post(&url).send().await {
        Ok(resp) => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let restart = json
                .get("restart_required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if restart {
                let data = OutputData::Message(format!(
                    "scheduled. Restart server to apply.\n{}",
                    json.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Operation scheduled")
                ));
                println!("{}", format_output(&data, *fmt));
            } else {
                let data = OutputData::Message(
                    json.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Engine added")
                        .to_string(),
                );
                println!("{}", format_output(&data, *fmt));
            }
        }
        Err(e) => {
            let data = OutputData::Error(format!("Request failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_remove(
    engine_type: String,
    server: String,
    force: bool,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/engine/{}/remove", server, engine_type);
    let body = serde_json::json!({ "drop_data": force });
    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Message(
                json.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Engine removal scheduled")
                    .to_string(),
            );
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Message(format!(
                "HTTP {} — {}\n\
                 To remove engine '{}' manually:\n\
                 1. Remove it from primusdb.toml [storage.engines]\n\
                 2. Restart: primusdb server restart --server {}\n\
                 {}3. Use --force to also drop data on disk.",
                status,
                text,
                engine_type,
                server,
                if force { "" } else { "  " },
            ));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Message(format!(
                "Could not reach server ({}).\n\
                 To remove engine '{}' manually:\n\
                 1. Remove it from primusdb.toml [storage.engines]\n\
                 2. Restart: primusdb server restart --server {}\n\
                 {}3. Use --force to also drop data on disk.",
                e,
                engine_type,
                server,
                if force { "" } else { "  " },
            ));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_upgrade(engine_type: String, server: String, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/engine/{}/upgrade", server, engine_type);
    match client.post(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Message(
                json.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Engine upgrade scheduled")
                    .to_string(),
            );
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Message(format!(
                "HTTP {} — {}\n\
                 To upgrade engine '{}' manually:\n\
                 1. Check current version: primusdb server status --server {}\n\
                 2. Upgrade the PrimusDB server binary\n\
                 3. Restart: primusdb server restart --server {}\n\
                 4. The server will migrate engine data on startup if needed.",
                status, text, engine_type, server, server,
            ));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Message(format!(
                "Could not reach server ({}).\n\
                 To upgrade engine '{}' manually:\n\
                 1. Check current version: primusdb server status --server {}\n\
                 2. Upgrade the PrimusDB server binary\n\
                 3. Restart: primusdb server restart --server {}\n\
                 4. The server will migrate engine data on startup if needed.",
                e, engine_type, server, server,
            ));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}
