//! Database management subcommands (`db list`, `db create`, `db drop`, ...).
//!
//! All operations run in client mode: they issue HTTP requests against
//! `GlobalArgs.server_url` and render the response through `fmt`.

use crate::cli::command::{DbSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;
use serde_json::json;

/// Dispatch a `db` subcommand to its handler.
pub async fn handle_db(cmd: DbSubcommands, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    match cmd {
        DbSubcommands::List { all, engine } => cmd_list(all, engine, global, fmt).await,
        DbSubcommands::Create {
            name,
            engine,
            namespace,
        } => cmd_create(name, engine, namespace, global, fmt).await,
        DbSubcommands::Drop { name, force } => cmd_drop(name, force, global, fmt).await,
        DbSubcommands::Describe { name, schema } => cmd_describe(name, schema, global, fmt).await,
        DbSubcommands::Use { name } => cmd_use(name, global, fmt).await,
    }
}

async fn cmd_list(
    _all: bool,
    _engine: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/databases", global.server_url);

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

async fn cmd_create(
    name: String,
    engine: String,
    namespace: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/databases", global.server_url);

    let body = json!({
        "name": name,
        "engines": [engine],
        "namespace": namespace,
    });

    match client.post(&url).json(&body).send().await {
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

async fn cmd_drop(
    name: String,
    _force: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/namespaces/{}", global.server_url, name);

    match client.delete(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data = OutputData::Message(format!("Database '{}' dropped", name));
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

async fn cmd_describe(
    name: String,
    _schema: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/namespaces/{}/resources", global.server_url, name);

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

async fn cmd_use(name: String, _global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let data = OutputData::Message(format!("Switched to database '{}'", name));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
