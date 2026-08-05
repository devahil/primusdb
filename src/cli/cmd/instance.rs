//! Instance management subcommands (`instance list`, `discover`, `inspect`,
//! `connect`, `stop`, `logs`).
//!
//! Lists/discovery probe local endpoints via the [`crate::cli::discovery`]
//! module; `connect` hands off to the interactive REPL.

use std::path::PathBuf;
use std::time::Duration;

use crate::cli::command::{GlobalArgs, InstanceSubcommands};
use crate::cli::discovery::{self, DiscoveryConfig};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch an `instance` subcommand to its handler.
pub async fn handle_instance(
    cmd: InstanceSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        InstanceSubcommands::List { all: _, format: _ } => cmd_list(fmt).await,
        InstanceSubcommands::Discover {
            host,
            start_port,
            max_ports,
            timeout,
        } => cmd_discover(host, start_port, max_ports, timeout, fmt).await,
        InstanceSubcommands::Inspect { endpoint, verbose } => {
            cmd_inspect(endpoint, verbose, global, fmt).await
        }
        InstanceSubcommands::Connect { endpoint, timeout } => {
            cmd_connect(endpoint, timeout, fmt).await
        }
        InstanceSubcommands::Stop { endpoint, force } => cmd_stop(endpoint, force, fmt).await,
        InstanceSubcommands::Logs {
            endpoint,
            lines,
            follow,
        } => cmd_logs(endpoint, lines, follow, fmt).await,
    }
}

async fn cmd_list(fmt: &OutputFormat) -> Result<()> {
    let config = DiscoveryConfig {
        ports: (8080..8085).collect(),
        timeout_ms: 3000,
        scan_localhost: true,
        check_config_files: true,
        check_processes: true,
    };

    let instances = discovery::discover_local(&config).await;

    if instances.is_empty() {
        let data = OutputData::Message("No PrimusDB instances found.".into());
        println!("{}", format_output(&data, *fmt));
    } else {
        let headers = vec![
            "Endpoint".into(),
            "Node ID".into(),
            "Version".into(),
            "Status".into(),
        ];
        let rows: Vec<Vec<String>> = instances
            .iter()
            .map(|i| {
                vec![
                    i.endpoint.clone(),
                    i.node_id.clone().unwrap_or_default(),
                    i.version.clone().unwrap_or_default(),
                    i.status.clone(),
                ]
            })
            .collect();
        let data = OutputData::Table { headers, rows };
        println!("{}", format_output(&data, *fmt));
    }

    Ok(())
}

async fn cmd_discover(
    host: String,
    start_port: u16,
    max_ports: u16,
    timeout: u64,
    fmt: &OutputFormat,
) -> Result<()> {
    let ports: Vec<u16> = (start_port..start_port + max_ports).collect();
    let config = DiscoveryConfig {
        ports,
        timeout_ms: timeout * 1000,
        scan_localhost: host == "127.0.0.1" || host == "localhost",
        check_config_files: false,
        check_processes: false,
    };

    let instances = discovery::discover_local(&config).await;

    if instances.is_empty() {
        let data = OutputData::Message(format!(
            "No PrimusDB instances found on {}:{}-{}.",
            host,
            start_port,
            start_port + max_ports - 1
        ));
        println!("{}", format_output(&data, *fmt));
    } else {
        let headers = vec![
            "Endpoint".into(),
            "Node ID".into(),
            "Version".into(),
            "Status".into(),
        ];
        let rows: Vec<Vec<String>> = instances
            .iter()
            .map(|i| {
                vec![
                    i.endpoint.clone(),
                    i.node_id.clone().unwrap_or_default(),
                    i.version.clone().unwrap_or_default(),
                    i.status.clone(),
                ]
            })
            .collect();
        let data = OutputData::Table { headers, rows };
        println!("{}", format_output(&data, *fmt));
    }

    Ok(())
}

async fn cmd_inspect(
    endpoint: String,
    verbose: bool,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let url = if endpoint.starts_with("http") {
        endpoint.clone()
    } else {
        format!("http://{}", endpoint)
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

    let mut rows = vec![vec!["Endpoint".into(), url.clone()]];

    // Health check
    match client.get(format!("{}/health", url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            rows.push(vec!["Health".into(), "healthy".into()]);
            if verbose {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(obj) = json.as_object() {
                        for (k, v) in obj {
                            rows.push(vec![format!("health.{}", k), v.to_string()]);
                        }
                    }
                }
            }
        }
        Ok(resp) => {
            rows.push(vec!["Health".into(), format!("HTTP {}", resp.status())]);
        }
        Err(_) => {
            rows.push(vec!["Health".into(), "unreachable".into()]);
        }
    }

    // Status check
    match client.get(format!("{}/status", url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(obj) = json.as_object() {
                    for (k, v) in obj {
                        if k != "health" || verbose {
                            rows.push(vec![format!("status.{}", k), v.to_string()]);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    let data = OutputData::Table {
        headers: vec!["Key".into(), "Value".into()],
        rows,
    };
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn cmd_connect(endpoint: String, _timeout: u64, _fmt: &OutputFormat) -> Result<()> {
    crate::cli::repl::run(crate::cli::repl::ReplState::new(Some(endpoint)))
}

async fn cmd_stop(endpoint: String, force: bool, fmt: &OutputFormat) -> Result<()> {
    let url = if endpoint.starts_with("http") {
        format!("{}/stop", endpoint)
    } else {
        format!("http://{}/stop", endpoint)
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

    let method = if force { "DELETE" } else { "POST" };
    let req = client
        .request(
            match method {
                "DELETE" => reqwest::Method::DELETE,
                _ => reqwest::Method::POST,
            },
            &url,
        )
        .send()
        .await;

    match req {
        Ok(resp) if resp.status().is_success() => {
            let data = OutputData::Message(format!("Stop signal sent to {}", endpoint));
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let data = OutputData::Message(format!(
                "Stop signal sent (HTTP {}). If server does not stop, use --force or kill manually.",
                resp.status()
            ));
            println!("{}", format_output(&data, *fmt));
        }
        Err(_e) => {
            // Fallback: try to find PID and kill
            let port = endpoint
                .split(':')
                .next_back()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(8080);

            match find_and_kill_process(port, force) {
                Ok(_) => {
                    let data = OutputData::Message(format!("Stopped process on port {}", port));
                    println!("{}", format_output(&data, *fmt));
                }
                Err(msg) => {
                    let data = OutputData::Error(format!(
                        "Could not stop {}: {}. Try: kill $(lsof -ti :{})",
                        endpoint, msg, port
                    ));
                    println!("{}", format_output(&data, *fmt));
                }
            }
        }
    }
    Ok(())
}

async fn cmd_logs(endpoint: String, lines: u32, follow: bool, fmt: &OutputFormat) -> Result<()> {
    // Try multiple log sources in order:
    // 1. journalctl (systemd)
    // 2. log file in data dir or /var/log/primusdb
    // 3. server output file in current directory
    let log_paths = [
        "/var/log/primusdb/primusdb.log",
        "primusdb.log",
        "logs/primusdb.log",
        "data/primusdb.log",
    ];

    // Try journalctl first
    let has_journalctl = std::process::Command::new("sh")
        .arg("-c")
        .arg("which journalctl 2>/dev/null")
        .output()
        .ok()
        .is_some_and(|o| o.status.success());

    if has_journalctl {
        let mut cmd = tokio::process::Command::new("journalctl");
        cmd.arg("-u")
            .arg("primusdb")
            .arg("--no-pager")
            .arg("-n")
            .arg(lines.to_string());

        if follow {
            cmd.arg("-f");
        }

        // Also filter by endpoint/port if we can extract it
        if let Some(port) = endpoint
            .split(':')
            .next_back()
            .and_then(|s| s.parse::<u16>().ok())
        {
            cmd.arg(format!("_PORT={}", port));
        }

        let output = cmd.output().await;

        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                let data = OutputData::Message(text.to_string());
                println!("{}", format_output(&data, *fmt));
                return Ok(());
            }
            Ok(_) => {
                // journalctl had output but no matching entries — fall through
            }
            Err(_) => {
                // journalctl not available — fall through
            }
        }
    }

    // Try log files
    for path in &log_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            match tokio::fs::read_to_string(&p).await {
                Ok(contents) => {
                    let line_count = contents.lines().count();
                    let start = line_count.saturating_sub(lines as usize);
                    let excerpt: Vec<&str> = contents.lines().skip(start).collect();
                    let data = OutputData::Message(excerpt.join("\n"));
                    println!("{}", format_output(&data, *fmt));

                    if follow {
                        let data = OutputData::Message(
                            "Follow mode not available for file logs. Use: tail -f <logfile>"
                                .into(),
                        );
                        println!("{}", format_output(&data, *fmt));
                    }
                    return Ok(());
                }
                Err(_) => continue,
            }
        }
    }

    // Fallback: provide guidance
    let data = OutputData::Message(format!(
        "No logs found for '{}'. Try:\n  \
         journalctl -u primusdb -n {}\n  \
         tail -f /var/log/primusdb/primusdb.log\n  \
         Or connect to the server and check its output.",
        endpoint, lines
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

fn find_and_kill_process(port: u16, force: bool) -> std::result::Result<(), String> {
    let output = std::process::Command::new("lsof")
        .args(["-ti", &format!("tcp:{}", port)])
        .output()
        .map_err(|e| format!("lsof failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("No process found on port {}", port));
    }

    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if pid_str.is_empty() {
        return Err(format!("No process found on port {}", port));
    }

    let signal = if force { "9" } else { "15" };
    let status = std::process::Command::new("kill")
        .args([&format!("-{}", signal), &pid_str])
        .status()
        .map_err(|e| format!("kill failed: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("kill {} failed", pid_str))
    }
}
