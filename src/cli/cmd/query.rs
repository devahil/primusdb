use crate::cli::command::GlobalArgs;
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

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

pub async fn handle_query(
    query: Vec<String>,
    _database: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let sql = query.join(" ");
    post_query(&sql, global, fmt).await
}

pub async fn handle_sql_file(
    sql: Vec<String>,
    _database: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let sql_text = sql.join(" ");
    post_query(&sql_text, global, fmt).await
}

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
