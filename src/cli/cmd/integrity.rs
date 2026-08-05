//! Integrity subcommands (`integrity status`, `integrity verify`, ...).
//!
//! All operations run in client mode: they issue HTTP requests against
//! `GlobalArgs.server_url` and render the response through `fmt`.

use crate::cli::command::{GlobalArgs, IntegritySubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch an `integrity` subcommand to its handler.
pub async fn handle_integrity(
    cmd: IntegritySubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        IntegritySubcommands::Status => get_json("/api/v1/integrity/status", global, fmt).await,
        IntegritySubcommands::Verify { db } => {
            get_json(
                &format!("/api/v1/databases/{}/integrity/verify", db),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Records { db } => {
            get_json(
                &format!("/api/v1/databases/{}/integrity/records", db),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Genesis { db } => {
            get_json(
                &format!("/api/v1/databases/{}/integrity/genesis", db),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Checkpoints { db } => {
            get_json(
                &format!("/api/v1/databases/{}/integrity/checkpoints", db),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Checkpoint { db } => {
            post_json(
                &format!("/api/v1/databases/{}/integrity/checkpoints", db),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Pending => get_json("/api/v1/integrity/pending", global, fmt).await,
        IntegritySubcommands::Flush => post_json("/api/v1/integrity/pending", global, fmt).await,
        IntegritySubcommands::Quarantine => {
            get_json("/api/v1/integrity/quarantine", global, fmt).await
        }
        IntegritySubcommands::Release { db, sequence } => {
            delete(
                &format!("/api/v1/integrity/quarantine/{}/{}", db, sequence),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Evidence { db } => {
            get_json(
                &format!("/api/v1/databases/{}/integrity/reconcile/evidence", db),
                global,
                fmt,
            )
            .await
        }
        IntegritySubcommands::Reconcile {
            db,
            peer_url,
            max_records,
        } => reconcile(db, peer_url, max_records, global, fmt).await,
        IntegritySubcommands::Ledger => get_json("/api/v1/ledger/status", global, fmt).await,
    }
}

/// Fetches the peer's evidence and records, then reconciles against this node.
///
/// This is the integrity-first handshake: only when the evidence differs are
/// the peer's records transferred and compared. The repair plan is printed;
/// nothing is applied automatically.
async fn reconcile(
    db: String,
    peer_url: String,
    max_records: Option<u64>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let peer_base = peer_url.trim_end_matches('/').to_string();

    let evidence_path = format!("/api/v1/databases/{}/integrity/reconcile/evidence", db);
    let evidence_text = match client
        .get(format!("{}{}", peer_base, evidence_path))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
        Ok(resp) => {
            let data = OutputData::Error(format!(
                "Peer evidence fetch failed (HTTP {}): {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
        Err(e) => {
            let data = OutputData::Error(format!("Peer connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
    };

    let peer_evidence: serde_json::Value = match serde_json::from_str(&evidence_text)
        .unwrap_or_default()
    {
        serde_json::Value::Object(_) => serde_json::from_str(&evidence_text).unwrap_or_default(),
        _ => serde_json::from_str(&evidence_text).unwrap_or_default(),
    };
    let peer_evidence_obj = peer_evidence
        .get("data")
        .or_else(|| peer_evidence.get("evidence"))
        .cloned()
        .unwrap_or_default();

    let records_path = format!("/api/v1/databases/{}/integrity/records", db);
    let records_text = match client
        .get(format!("{}{}", peer_base, records_path))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
        Ok(resp) => {
            let data = OutputData::Error(format!(
                "Peer records fetch failed (HTTP {}): {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
        Err(e) => {
            let data = OutputData::Error(format!("Peer connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
    };

    let records_json: serde_json::Value = serde_json::from_str(&records_text).unwrap_or_default();
    let mut peer_records = records_json
        .get("data")
        .or_else(|| records_json.get("records"))
        .cloned()
        .unwrap_or_default();
    if let Some(limit) = max_records {
        if let Some(arr) = peer_records.as_array_mut() {
            arr.truncate(limit as usize);
        }
    }

    let reconcile_path = format!("/api/v1/databases/{}/integrity/reconcile", db);
    let body = serde_json::json!({
        "peer_records": peer_records,
        "peer_evidence": peer_evidence_obj,
    });
    match client
        .post(format!("{}{}", global.server_url, reconcile_path))
        .json(&body)
        .send()
        .await
    {
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

async fn get_json(path: &str, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", global.server_url, path);
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

async fn post_json(path: &str, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", global.server_url, path);
    match client.post(&url).send().await {
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

async fn delete(path: &str, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", global.server_url, path);
    match client.delete(&url).send().await {
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
