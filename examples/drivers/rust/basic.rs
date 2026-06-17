use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = "http://localhost:8080";
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    // Check health
    println!("=== PrimusDB Rust Example ===");

    match client.get(format!("{}/health", base_url)).send().await {
        Ok(resp) => {
            let health: Value = resp.json().await?;
            println!("Health: {}", health);
        }
        Err(e) => {
            eprintln!("Error connecting to PrimusDB: {}", e);
            std::process::exit(1);
        }
    }

    // Get version
    match client.get(format!("{}/version", base_url)).send().await {
        Ok(resp) => {
            let version: Value = resp.json().await?;
            println!("Version: {}", version);
        }
        Err(e) => {
            eprintln!("Error fetching version: {}", e);
        }
    }

    // Create a record
    let record = json!({
        "collection": "users",
        "data": {
            "name": "Alice",
            "email": "alice@example.com"
        }
    });

    match client
        .post(format!("{}/records", base_url))
        .json(&record)
        .send()
        .await
    {
        Ok(resp) => {
            let created: Value = resp.json().await?;
            println!("Created record: {}", created);
        }
        Err(e) => {
            eprintln!("Error creating record: {}", e);
        }
    }

    // Query records
    match client
        .get(format!("{}/records/users", base_url))
        .send()
        .await
    {
        Ok(resp) => {
            let records: Value = resp.json().await?;
            println!("Records: {}", records);
        }
        Err(e) => {
            eprintln!("Error querying records: {}", e);
        }
    }

    Ok(())
}
