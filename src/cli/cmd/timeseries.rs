//! Time series subcommands (`ts list`, `describe`, `query`, `aggregate`,
//! `downsample`, `retain`, `resolution`, `stats`).
//!
//! All operations run in client mode against the `/api/v1/timeseries/*`
//! endpoints on `GlobalArgs.server_url`.

use crate::cli::command::{GlobalArgs, TimeSeriesSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Parse a timestamp argument (unix ms, RFC 3339, or naive local) to ms.
fn parse_ts(v: &str) -> i64 {
    if let Ok(ts) = v.parse::<i64>() {
        return ts;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(v) {
        return dt.timestamp_millis();
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S") {
        return dt.and_utc().timestamp_millis();
    }
    chrono::Utc::now().timestamp_millis()
}

/// Dispatch a `ts` subcommand to its handler.
pub async fn handle_timeseries(
    cmd: TimeSeriesSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        TimeSeriesSubcommands::List { verbose } => cmd_list(verbose, global, fmt).await,
        TimeSeriesSubcommands::Describe { metric } => cmd_describe(metric, global, fmt).await,
        TimeSeriesSubcommands::Query {
            metric,
            start,
            end,
            tags,
            fields,
            limit,
            resolution,
        } => {
            cmd_query(
                metric, start, end, tags, fields, limit, resolution, global, fmt,
            )
            .await
        }
        TimeSeriesSubcommands::Aggregate {
            metric,
            start,
            end,
            tags,
            function,
            interval,
            fill,
        } => {
            cmd_aggregate(
                metric, start, end, tags, function, interval, fill, global, fmt,
            )
            .await
        }
        TimeSeriesSubcommands::Downsample {
            metric,
            source,
            target,
            function,
        } => cmd_downsample(metric, source, target, function, global, fmt).await,
        TimeSeriesSubcommands::Retain { metric } => cmd_retain(metric, global, fmt).await,
        TimeSeriesSubcommands::Resolution {
            metric,
            resolution,
            retention_days,
            agg_fn,
        } => cmd_resolution(metric, resolution, retention_days, agg_fn, global, fmt).await,
        TimeSeriesSubcommands::Stats => cmd_stats(global, fmt).await,
    }
}

async fn cmd_list(verbose: bool, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/timeseries/metrics", global.server_url);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            if verbose {
                let data = OutputData::Json(json);
                println!("{}", format_output(&data, *fmt));
            } else {
                let metrics = json
                    .get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
                    .unwrap_or_default();
                let names: Vec<String> = metrics
                    .iter()
                    .filter_map(|m| m.as_str().map(|s| s.to_string()))
                    .collect();
                let data = OutputData::List(names);
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

async fn cmd_describe(metric: String, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/timeseries/metrics/{}", global.server_url, metric);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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

// CLI command handler; each arg maps to an independent CLI flag.
#[allow(clippy::too_many_arguments)]
async fn cmd_query(
    metric: String,
    start: Option<String>,
    end: Option<String>,
    tags: Option<String>,
    fields: Option<String>,
    limit: u64,
    resolution: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/timeseries/{}/query", global.server_url, metric);

    let mut body = serde_json::json!({
        "metric": metric,
        "limit": limit,
    });
    if let Some(s) = start {
        body["start_time"] = serde_json::json!(parse_ts(&s));
    }
    if let Some(e) = end {
        body["end_time"] = serde_json::json!(parse_ts(&e));
    }
    if let Some(t) = tags {
        if let Ok(tags_map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&t)
        {
            body["tags"] = serde_json::to_value(tags_map).unwrap_or_default();
        }
    }
    if let Some(f) = fields {
        let fields_list: Vec<String> = f.split(',').map(|s| s.trim().to_string()).collect();
        body["fields"] = serde_json::to_value(fields_list).unwrap_or_default();
    }
    if let Some(r) = resolution {
        body["resolution"] = serde_json::json!(r);
    }

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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

// CLI command handler; each arg maps to an independent CLI flag.
#[allow(clippy::too_many_arguments)]
async fn cmd_aggregate(
    metric: String,
    start: Option<String>,
    end: Option<String>,
    tags: Option<String>,
    function: String,
    interval: String,
    fill: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/timeseries/{}/aggregate",
        global.server_url, metric
    );

    let mut body = serde_json::json!({
        "metric": metric,
        "aggregation": function,
        "resolution": interval,
    });
    if let Some(s) = start {
        body["start_time"] = serde_json::json!(parse_ts(&s));
    }
    if let Some(e) = end {
        body["end_time"] = serde_json::json!(parse_ts(&e));
    }
    if let Some(t) = tags {
        if let Ok(tags_map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&t)
        {
            body["tags"] = serde_json::to_value(tags_map).unwrap_or_default();
        }
    }
    if let Some(f) = fill {
        body["fill_policy"] = serde_json::json!(f);
    }

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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

async fn cmd_downsample(
    metric: String,
    source: String,
    target: String,
    function: String,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/timeseries/{}/downsample",
        global.server_url, metric
    );

    let body = serde_json::json!({
        "source_resolution": source,
        "target_resolution": target,
        "agg_fn": function,
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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

async fn cmd_retain(metric: String, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/timeseries/{}/retain", global.server_url, metric);

    match client.post(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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

async fn cmd_resolution(
    metric: String,
    resolution: String,
    retention_days: u32,
    agg_fn: String,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/timeseries/{}/resolution",
        global.server_url, metric
    );

    let body = serde_json::json!({
        "resolution": resolution,
        "retention_days": retention_days,
        "agg_fn": agg_fn,
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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

async fn cmd_stats(global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/timeseries/stats", global.server_url);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
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
