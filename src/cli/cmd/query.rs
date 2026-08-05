//! Query subcommands (`query`, `sql`, `explain`).
//!
//! All three post the given text to the server's `/api/v1/uql` endpoint in
//! client mode and render the JSON response through `fmt`.

use crate::cli::command::GlobalArgs;
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// POST a SQL statement to the server's UQL endpoint.
async fn post_query(sql: &str, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/uql", global.server_url);
    let body = serde_json::json!({"query": sql, "language": "sql"});

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

/// Execute a raw query against the connected server.
pub async fn handle_query(
    query: Vec<String>,
    _database: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let sql = query.join(" ");
    post_query(&sql, global, fmt).await
}

/// Execute a SQL query against the connected server.
pub async fn handle_sql_file(
    sql: Vec<String>,
    _database: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let sql_text = sql.join(" ");
    post_query(&sql_text, global, fmt).await
}

/// Request an `EXPLAIN` plan for a query without executing it.
pub async fn handle_explain(
    query: Vec<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let sql = query.join(" ");
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/uql", global.server_url);
    let body = serde_json::json!({"query": format!("EXPLAIN {}", sql), "language": "sql"});

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
