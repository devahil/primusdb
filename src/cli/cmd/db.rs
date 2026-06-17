use crate::cli::command::{DbSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

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
    let url = format!("{}/api/v1/info-schema/relational/tables", global.server_url);

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
    _namespace: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/table/{}/{}/create",
        global.server_url, engine, name
    );

    match client.post(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data = OutputData::Message(format!("Database '{}' created", name));
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
    let url = format!(
        "{}/api/v1/table/relational/{}/drop",
        global.server_url, name
    );

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
    let url = format!(
        "{}/api/v1/table/relational/{}/info",
        global.server_url, name
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

async fn cmd_use(name: String, _global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let data = OutputData::Message(format!("Switched to database '{}'", name));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
