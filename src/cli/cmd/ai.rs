//! AI/ML subcommands (`ai models`, `train`, `predict`, `analyze`,
//! `anomalies`).
//!
//! `predict` and `analyze` call the server's advanced AI endpoints; the
//! remaining subcommands are not yet wired to the CLI.

use crate::cli::command::{AiSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch an `ai` subcommand to its handler.
pub async fn handle_ai(cmd: AiSubcommands, _global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    match cmd {
        AiSubcommands::Models { .. }
        | AiSubcommands::Train { .. }
        | AiSubcommands::Anomalies { .. } => {
            let data = OutputData::Message(
                "This AI subcommand is not yet available via CLI. \
                 Use the REST API at POST /api/v1/advanced/{analyze,predict}/:storage_type/:table \
                 or use the TUI AI/ML section."
                    .to_string(),
            );
            println!("{}", format_output(&data, *fmt));
            Ok(())
        }
        AiSubcommands::Predict {
            model,
            input,
            raw,
            top_k,
        } => cmd_predict(model, input, raw, top_k, _global, fmt).await,
        AiSubcommands::Analyze {
            table,
            columns,
            analysis_type,
        } => cmd_analyze(table, columns, analysis_type, _global, fmt).await,
    }
}

async fn cmd_predict(
    _model: String,
    input: String,
    _raw: bool,
    _top_k: u32,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/advanced/predict/relational/{}",
        global.server_url, input
    );
    let body = serde_json::json!({"data": {}, "prediction_type": "regression"});

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

async fn cmd_analyze(
    table: String,
    _columns: Option<String>,
    analysis_type: String,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/advanced/analyze/relational/{}",
        global.server_url, table
    );
    let body = serde_json::json!({
        "analysis_type": analysis_type,
        "conditions": {}
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
