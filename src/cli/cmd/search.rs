//! Unified search subcommands (`search text`, `search vector`).
//!
//! Both subcommands run in client mode: they issue a GET against
//! `/api/v1/search` with the appropriate query parameters and render the
//! merged, ranked hits through `fmt`.

use crate::cli::command::{GlobalArgs, SearchSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch a `search` subcommand to its handler.
pub async fn handle_search(
    cmd: SearchSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        SearchSubcommands::Text {
            query,
            storage_types,
            tables,
            mode,
            limit,
            offset,
        } => {
            let q = query.join(" ");
            let mut params: Vec<String> = vec![
                ("q", q),
                ("mode", mode),
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
            ]
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(&v)))
            .collect();
            if let Some(st) = storage_types {
                params.push(format!("storage_types={}", urlencode(&st)));
            }
            if let Some(t) = tables {
                params.push(format!("tables={}", urlencode(&t)));
            }
            get_json(&params.join("&"), global, fmt).await
        }
        SearchSubcommands::Vector {
            query_vector,
            tables,
            limit,
            offset,
        } => {
            let mut params: Vec<String> = vec![
                ("query_vector", query_vector),
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
            ]
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(&v)))
            .collect();
            if let Some(t) = tables {
                params.push(format!("tables={}", urlencode(&t)));
            }
            get_json(&params.join("&"), global, fmt).await
        }
    }
}

async fn get_json(query_string: &str, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/search?{}", global.server_url, query_string);
    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            render(status, text, fmt);
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

fn render(status: reqwest::StatusCode, text: String, fmt: &OutputFormat) {
    if status.is_success() {
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let data = OutputData::Json(json);
        println!("{}", format_output(&data, *fmt));
    } else {
        let data = OutputData::Error(format!("HTTP {}: {}", status, text));
        println!("{}", format_output(&data, *fmt));
    }
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".to_string(),
            ',' => "%2C".to_string(),
            '[' => "%5B".to_string(),
            ']' => "%5D".to_string(),
            '"' => "%22".to_string(),
            '{' => "%7B".to_string(),
            '}' => "%7D".to_string(),
            ':' => "%3A".to_string(),
            '=' => "%3D".to_string(),
            '&' => "%26".to_string(),
            '?' => "%3F".to_string(),
            '#' => "%23".to_string(),
            other => other.to_string(),
        })
        .collect()
}
