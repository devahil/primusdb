//! Vector subcommands (`vector search`, `index`, `stats`, `compact`).
//!
//! `search` calls the server's vector-search endpoint; the remaining
//! subcommands are not yet wired to the CLI.

use crate::cli::command::{GlobalArgs, VectorSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch a `vector` subcommand to its handler.
pub async fn handle_vector(
    cmd: VectorSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        VectorSubcommands::Search {
            index, vector, k, ..
        } => cmd_search(index, vector, k, global, fmt).await,
        VectorSubcommands::Index { .. }
        | VectorSubcommands::Stats { .. }
        | VectorSubcommands::Compact { .. } => {
            let data = OutputData::Message(
                "This vector subcommand is not yet available via CLI. \
                 Use the REST API at POST /api/v1/advanced/vector-search/:table \
                 or use the TUI Vector Indexes section."
                    .to_string(),
            );
            println!("{}", format_output(&data, *fmt));
            Ok(())
        }
    }
}

/// Run a vector similarity search against `POST /api/v1/advanced/vector-search/:table`.
async fn cmd_search(
    index: String,
    vector: String,
    k: u32,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/advanced/vector-search/{}",
        global.server_url, index
    );
    let query_vector: Vec<f64> = vector
        .split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();
    let body = serde_json::json!({
        "query_vector": query_vector,
        "limit": k
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
