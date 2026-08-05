//! Namespace subcommands (`namespace list`, `create`, `drop`, `describe`,
//! `policy`).
//!
//! All operations run in client mode against the `/api/v1/namespaces/*`
//! endpoints on `GlobalArgs.server_url`.

use crate::cli::command::{GlobalArgs, NamespaceSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch a `namespace` subcommand to its handler.
pub async fn handle_namespace(
    cmd: NamespaceSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        NamespaceSubcommands::List { parent, full_paths } => {
            cmd_list(parent, full_paths, global, fmt).await
        }
        NamespaceSubcommands::Create {
            path,
            description,
            parent,
            quota,
        } => cmd_create(path, description, parent, quota, global, fmt).await,
        NamespaceSubcommands::Drop {
            path,
            recursive,
            force,
        } => cmd_drop(path, recursive, force, global, fmt).await,
        NamespaceSubcommands::Describe { path, resources } => {
            cmd_describe(path, resources, global, fmt).await
        }
        NamespaceSubcommands::Policy {
            path,
            set,
            unset,
            list,
        } => cmd_policy(path, set, unset, list, global, fmt).await,
    }
}

async fn cmd_list(
    _parent: Option<String>,
    _full_paths: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/namespaces", global.server_url);

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
    path: String,
    description: Option<String>,
    _parent: Option<String>,
    _quota: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/namespaces/{}", global.server_url, path);
    let mut body = serde_json::json!({});
    if let Some(desc) = description {
        body["description"] = serde_json::Value::String(desc);
    }

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data = OutputData::Message(format!("Namespace '{}' created", path));
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
    path: String,
    _recursive: bool,
    _force: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/namespaces/{}", global.server_url, path);

    match client.delete(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data = OutputData::Message(format!("Namespace '{}' dropped", path));
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
    path: String,
    _resources: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/namespaces/{}", global.server_url, path);

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

async fn cmd_policy(
    path: String,
    _set: Option<Vec<String>>,
    _unset: Option<String>,
    _list: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/namespaces/{}/effective-policy",
        global.server_url, path
    );

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
