//! Network discovery subcommand (`discover`).
//!
//! Probes a single port for PrimusDB instances using
//! [`crate::cli::discovery::discover_local`].

use crate::cli::discovery::{self, DiscoveryConfig};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Probe the given broadcast/port for PrimusDB instances.
pub async fn handle_discover(
    broadcast: String,
    port: u16,
    timeout: u64,
    fmt: &OutputFormat,
) -> Result<()> {
    let config = DiscoveryConfig {
        ports: vec![port],
        timeout_ms: timeout * 1000,
        scan_localhost: broadcast == "127.0.0.1" || broadcast == "255.255.255.255",
        check_config_files: false,
        check_processes: false,
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
