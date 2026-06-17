use std::path::PathBuf;
use std::time::Duration;

use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Diagnostic check result
#[allow(dead_code)]
struct Check {
    name: String,
    status: CheckStatus,
    detail: String,
}

#[allow(dead_code)]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Info,
}

impl Check {
    fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Check {
            name: name.into(),
            status: CheckStatus::Pass,
            detail: detail.into(),
        }
    }
    fn warn(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Check {
            name: name.into(),
            status: CheckStatus::Warn,
            detail: detail.into(),
        }
    }
    #[allow(dead_code)]
    fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Check {
            name: name.into(),
            status: CheckStatus::Fail,
            detail: detail.into(),
        }
    }
    fn info(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Check {
            name: name.into(),
            status: CheckStatus::Info,
            detail: detail.into(),
        }
    }
}

pub async fn handle_doctor(
    aggressive: bool,
    report: Option<PathBuf>,
    fmt: &OutputFormat,
) -> Result<()> {
    let mut checks: Vec<Check> = Vec::new();

    // ── Rust version ──────────────────────────────────
    let rust_v = std::env::var("CARGO_PKG_RUST_VERSION")
        .ok()
        .unwrap_or_else(|| {
            std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_else(|_| {
                let out = std::process::Command::new("rustc")
                    .arg("--version")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok());
                out.unwrap_or_else(|| "unknown".into())
            })
        });
    checks.push(Check::info("Rust toolchain", rust_v.trim().to_string()));

    // ── Binary version ────────────────────────────────
    checks.push(Check::info(
        "PrimusDB version",
        env!("CARGO_PKG_VERSION").to_string(),
    ));

    // ── Binary build info ─────────────────────────────
    checks.push(Check::info("Build profile", {
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    }));
    checks.push(Check::info("License", env!("CARGO_PKG_LICENSE")));

    // ── Config file ───────────────────────────────────
    let config_paths = [
        PathBuf::from("primusdb.toml"),
        PathBuf::from("config.toml"),
        PathBuf::from("config/primusdb.toml"),
        PathBuf::from("~/.config/primusdb/config.toml"),
    ];

    let mut found_config = None;
    for p in &config_paths {
        if p.exists() {
            found_config = Some(p.clone());
            break;
        }
    }

    match &found_config {
        Some(p) => checks.push(Check::pass("Config file", p.display().to_string())),
        None => checks.push(Check::warn(
            "Config file",
            "Not found in default locations (primusdb.toml, config.toml)",
        )),
    }

    // ── Data directory ────────────────────────────────
    let data_dirs = [PathBuf::from("data"), PathBuf::from("/tmp/primusdb_data")];

    for dir in &data_dirs {
        if dir.exists() {
            let readable = dir.is_dir();
            let writable = std::fs::metadata(dir)
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false);
            if readable && writable {
                checks.push(Check::pass("Data directory", dir.display().to_string()));
            } else {
                checks.push(Check::warn(
                    "Data directory",
                    format!("{} (permission issue)", dir.display()),
                ));
            }
            break;
        }
    }

    if !checks.iter().any(|c| c.name == "Data directory") {
        checks.push(Check::info(
            "Data directory",
            "Not created yet (will be created on first start)",
        ));
    }

    // ── Port availability ─────────────────────────────
    let default_port: u16 = 8080;
    let port_available = check_port(default_port);
    if port_available {
        checks.push(Check::pass("Port 8080", "Available"));
    } else {
        checks.push(Check::warn("Port 8080", "In use or unavailable"));
    }

    // ── Server health probe ───────────────────────────
    let health_url = "http://127.0.0.1:8080/health";
    if let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        match client.get(health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                checks.push(Check::pass("Server health", "Running and responding"));
            }
            Ok(resp) => {
                checks.push(Check::warn(
                    "Server health",
                    format!("Responded with HTTP {}", resp.status()),
                ));
            }
            Err(_) => {
                checks.push(Check::warn(
                    "Server health",
                    "Not reachable at http://127.0.0.1:8080",
                ));
            }
        }

        // ── Status endpoint ───────────────────────────────
        let status_url = "http://127.0.0.1:8080/status";
        match client.get(status_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                checks.push(Check::pass("Status endpoint", "Responds correctly"));
            }
            Ok(resp) => {
                checks.push(Check::warn(
                    "Status endpoint",
                    format!("HTTP {}", resp.status()),
                ));
            }
            Err(_) => {
                checks.push(Check::warn("Status endpoint", "Not reachable"));
            }
        }

        // ── Metrics endpoint ──────────────────────────────
        if aggressive {
            let metrics_url = "http://127.0.0.1:8080/metrics";
            match client.get(metrics_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    checks.push(Check::pass("Metrics endpoint", "Available"));
                }
                Ok(resp) => {
                    checks.push(Check::warn(
                        "Metrics endpoint",
                        format!("HTTP {}", resp.status()),
                    ));
                }
                Err(_) => {
                    checks.push(Check::warn("Metrics endpoint", "Not reachable"));
                }
            }
        }
    } else {
        checks.push(Check::warn("Server health", "Could not create HTTP client"));
    }

    // ── Docker availability ───────────────────────────
    let has_docker = std::process::Command::new("docker")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some();
    if has_docker {
        checks.push(Check::pass("Docker", "Available"));
    } else {
        checks.push(Check::info("Docker", "Not detected"));
    }

    // ── Disk space ────────────────────────────────────
    if aggressive {
        // Use `df` command (POSIX) to get disk space info
        match std::process::Command::new("df").arg("-h").arg(".").output() {
            Ok(out) if out.status.success() => {
                let output_str = String::from_utf8_lossy(&out.stdout);
                let line = output_str.lines().nth(1).unwrap_or("");
                let free = line.split_whitespace().nth(3).unwrap_or("unknown");
                checks.push(Check::pass(
                    "Disk space",
                    format!("{} free (current dir)", free),
                ));
            }
            _ => {
                checks.push(Check::info("Disk space", "Could not determine"));
            }
        }
    }

    // ── OS info ───────────────────────────────────────
    checks.push(Check::info("OS", std::env::consts::OS));
    checks.push(Check::info("Architecture", std::env::consts::ARCH));

    // ── Output ────────────────────────────────────────
    let rows: Vec<Vec<String>> = checks
        .iter()
        .map(|c| {
            let status_str = match c.status {
                CheckStatus::Pass => "✓".to_string(),
                CheckStatus::Warn => "~".to_string(),
                CheckStatus::Fail => "✗".to_string(),
                CheckStatus::Info => "●".to_string(),
            };
            vec![status_str, c.name.clone(), c.detail.clone()]
        })
        .collect();

    let headers = vec!["".into(), "Check".into(), "Result".into()];
    let data = OutputData::Table { headers, rows };
    println!("{}", format_output(&data, *fmt));

    // ── Write report ──────────────────────────────────
    if let Some(path) = report {
        let report_content: Vec<String> = checks
            .iter()
            .map(|c| {
                format!(
                    "{}: {} — {}",
                    c.name,
                    match c.status {
                        CheckStatus::Pass => "PASS",
                        CheckStatus::Warn => "WARN",
                        CheckStatus::Fail => "FAIL",
                        CheckStatus::Info => "INFO",
                    },
                    c.detail
                )
            })
            .collect();
        tokio::fs::write(&path, report_content.join("\n")).await?;
    }

    Ok(())
}

fn check_port(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}
