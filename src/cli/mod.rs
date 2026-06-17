pub mod cmd;
pub mod command;
pub mod discovery;
pub mod output;
pub mod tui;

#[allow(dead_code)]
mod legacy;

use std::path::PathBuf;

use clap::CommandFactory;
use clap::Parser;
pub use command::{Cli, Commands};
pub use output::{ExitCode, OutputData, OutputFormat};

use crate::Result;
use command::{BenchSubcommands, GlobalArgs, GovernorSubcommands, MigrateSubcommands};
use output::format_output;

/// Main entry point for the PrimusDB CLI.
///
/// Parses command-line arguments and dispatches to the appropriate handler.
pub async fn run() -> Result<()> {
    run_cli(Cli::parse()).await
}

/// Dispatch a pre-parsed Cli to the appropriate handler.
pub async fn run_cli(cli: Cli) -> Result<()> {
    let fmt: OutputFormat = cli.global.format.parse().unwrap_or(OutputFormat::Table);

    match cli.command {
        Commands::Server(cmd) => handle_server(cmd, &fmt).await,
        Commands::Connect { server, timeout } => handle_connect(server, timeout, &fmt).await,
        Commands::Health => handle_health(&cli.global, &fmt).await,
        Commands::Status => handle_status(&cli.global, &fmt).await,
        Commands::Instance(cmd) => handle_instance(cmd, &cli.global, &fmt).await,
        Commands::Tui { server, no_color } => handle_tui(server, no_color).await,
        Commands::Query { query, database } => {
            handle_query(query, database, &cli.global, &fmt).await
        }
        Commands::Sql { sql, database } => handle_sql_file(sql, database, &cli.global, &fmt).await,
        Commands::Db(cmd) => handle_db(cmd, &cli.global, &fmt).await,
        Commands::Engine(cmd) => handle_engine(cmd, &cli.global, &fmt).await,
        Commands::Namespace(cmd) => handle_namespace(cmd, &cli.global, &fmt).await,
        Commands::Config(cmd) => handle_config(cmd, &cli.global, &fmt).await,
        Commands::Cluster(cmd) => handle_cluster(cmd, &cli.global, &fmt).await,
        Commands::Protocol(cmd) => handle_protocol(cmd, &cli.global, &fmt).await,
        Commands::Backup(cmd) => handle_backup(cmd, &cli.global, &fmt).await,
        Commands::Restore {
            source,
            database: db,
            force,
        } => handle_restore(source, db, force, &cli.global, &fmt).await,
        Commands::Metrics {
            filter,
            watch,
            interval,
        } => handle_metrics(filter, watch, interval, &cli.global, &fmt).await,
        Commands::Auth(cmd) => handle_auth(cmd, &cli.global, &fmt).await,
        Commands::User(cmd) => handle_user(cmd, &cli.global, &fmt).await,
        Commands::Role(cmd) => handle_role(cmd, &cli.global, &fmt).await,
        Commands::Ai(cmd) => handle_ai(cmd, &cli.global, &fmt).await,
        Commands::Vector(cmd) => handle_vector(cmd, &cli.global, &fmt).await,
        Commands::Graph(cmd) => handle_graph(cmd, &cli.global, &fmt).await,
        Commands::Cdc(cmd) => handle_cdc(cmd, &cli.global, &fmt).await,
        Commands::Explain { query } => handle_explain(query, &cli.global, &fmt).await,
        Commands::Bench(cmd) => handle_bench(cmd, &cli.global, &fmt).await,
        Commands::Migrate(cmd) => handle_migrate(cmd, &cli.global, &fmt).await,
        Commands::Doctor { aggressive, report } => {
            handle_doctor(aggressive, report, &cli.global, &fmt).await
        }
        Commands::Discover {
            broadcast,
            port,
            timeout,
        } => handle_discover(broadcast, port, timeout, &fmt).await,
        Commands::Governor(cmd) => handle_governor(cmd, &cli.global, &fmt).await,
        Commands::Completion { shell } => handle_completion(shell),
        Commands::Version { verbose } => handle_version(verbose),
    }
}

// ---------------------------------------------------------------------------
// Handler functions
// ---------------------------------------------------------------------------

async fn handle_server(cmd: command::ServerSubcommands, fmt: &OutputFormat) -> Result<()> {
    cmd::server::handle_server(cmd, fmt).await
}

async fn handle_connect(server: Option<String>, timeout: u64, fmt: &OutputFormat) -> Result<()> {
    let url = server.unwrap_or_else(|| "http://localhost:8080".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

    match client.get(format!("{}/health", url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            let data = OutputData::Message(format!("Connected to {}", url));
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let data = OutputData::Error(format!("Connection failed: HTTP {}", resp.status()));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn handle_health(global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

    match client
        .get(format!("{}/health", global.server_url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let data = OutputData::Error(format!("Health check failed: HTTP {}", resp.status()));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Health check failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn handle_status(global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

    match client
        .get(format!("{}/status", global.server_url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let data = OutputData::Error(format!("Status check failed: HTTP {}", resp.status()));
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Status check failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn handle_instance(
    cmd: command::InstanceSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::instance::handle_instance(cmd, global, fmt).await
}

async fn handle_tui(server: Option<String>, _no_color: bool) -> Result<()> {
    match server {
        Some(url) => tui::run_tui_connect(&url).await,
        None => tui::run_tui().await,
    }
}

async fn handle_query(
    query: Vec<String>,
    database: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::query::handle_query(query, database, global, fmt).await
}

async fn handle_sql_file(
    sql: Vec<String>,
    database: Option<String>,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::query::handle_sql_file(sql, database, global, fmt).await
}

async fn handle_db(
    cmd: command::DbSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::db::handle_db(cmd, global, fmt).await
}

async fn handle_engine(
    cmd: command::EngineSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::engine::handle_engine(cmd, global, fmt).await
}

async fn handle_namespace(
    cmd: command::NamespaceSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::namespace::handle_namespace(cmd, global, fmt).await
}

async fn handle_cluster(
    cmd: command::ClusterSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::cluster::handle_cluster(cmd, global, fmt).await
}

async fn handle_protocol(
    cmd: command::ProtocolSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::protocol::handle_protocol(cmd, global, fmt).await
}

async fn handle_backup(
    cmd: command::BackupSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::backup::handle_backup(cmd, global, fmt).await
}

async fn handle_restore(
    source: PathBuf,
    database: Option<String>,
    force: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::backup::handle_restore(source, database, force, global, fmt).await
}

async fn handle_metrics(
    _filter: Option<String>,
    _watch: bool,
    _interval: u64,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let data = OutputData::Error(format!("Failed to create HTTP client: {}", e));
            println!("{}", format_output(&data, *fmt));
            return Ok(());
        }
    };

    match client
        .get(format!("{}/metrics", global.server_url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await.unwrap_or_default();
            let data = OutputData::Message(text);
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

async fn handle_auth(
    cmd: command::AuthSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::auth::handle_auth(cmd, global, fmt).await
}

async fn handle_user(
    cmd: command::UserSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::auth::handle_user(cmd, global, fmt).await
}

async fn handle_role(
    cmd: command::RoleSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::auth::handle_role(cmd, global, fmt).await
}

async fn handle_ai(
    cmd: command::AiSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::ai::handle_ai(cmd, global, fmt).await
}

async fn handle_vector(
    cmd: command::VectorSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::vector::handle_vector(cmd, global, fmt).await
}

async fn handle_graph(
    cmd: command::GraphSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::graph::handle_graph(cmd, global, fmt).await
}

async fn handle_cdc(
    cmd: command::CdcSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::cdc::handle_cdc(cmd, global, fmt).await
}

async fn handle_explain(query: Vec<String>, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    cmd::query::handle_explain(query, global, fmt).await
}

async fn handle_bench(
    cmd: BenchSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let feature = match &cmd {
        BenchSubcommands::Run { .. } => "bench run",
        BenchSubcommands::List { .. } => "bench list",
        BenchSubcommands::Report { .. } => "bench report",
    };
    let data = OutputData::Message(format!(
        "{} is not yet available via CLI. \
         Use the Criterion benchmark framework directly:\n  \
         cargo bench (run all benchmarks)\n  \
         cargo bench --bench <name> (run a specific benchmark)",
        feature
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn handle_migrate(
    cmd: MigrateSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        MigrateSubcommands::InspectSource {
            source,
            url,
            namespace,
            format: _rpt_fmt,
        } => {
            let data = OutputData::Message(format!(
                "Migration source inspection is available when the corresponding database driver crate\n\
                 (mysql, tokio-postgres, mongodb) is added as a dependency.\n\n\
                 Requested: source={}, url={}, namespace={:?}",
                source,
                crate::migration::report::MigrationReport::mask_url(&url),
                namespace
            ));
            println!("{}", format_output(&data, *fmt));
            Ok(())
        }
        MigrateSubcommands::Plan {
            source,
            url,
            target,
            namespace,
            mapping,
            mode,
            format: _rpt_fmt,
            output,
        } => {
            let plan = crate::migration::run_migrate_plan(
                source, url, target, namespace, mapping, mode, output,
            )?;
            let data = plan;
            println!("{}", format_output(&data, *fmt));
            Ok(())
        }
        MigrateSubcommands::Import {
            source,
            url,
            target,
            namespace,
            mapping,
            mode,
            batch_size,
            limit,
            include,
            exclude,
            overwrite,
            resume,
            yes,
        } => {
            let conn_str_for_log = crate::migration::report::MigrationReport::mask_url(&url);
            if !yes {
                println!("Migration Import Plan:");
                println!("  Source: {} at {}", source, conn_str_for_log);
                println!(
                    "  Target: {}",
                    target.as_deref().unwrap_or("http://localhost:8080")
                );
                println!("  Namespace: {}", namespace.as_deref().unwrap_or("default"));
                println!("  Mode: {}", mode);
                println!("  Overwrite: {}", if overwrite { "yes" } else { "no" });
                println!("  Resume: {}", if resume { "yes" } else { "no" });
                println!();
                print!("Proceed with import? [y/N]: ");
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                let input = input.trim().to_lowercase();
                if input != "y" && input != "yes" {
                    println!("Import cancelled.");
                    return Ok(());
                }
            }
            let cfg = crate::migration::MigrateImportConfig {
                source,
                url,
                target,
                namespace,
                mapping,
                mode,
                batch_size,
                limit,
                include,
                exclude,
                overwrite,
                resume,
            };
            let result = crate::migration::run_migrate_import(cfg)?;
            println!("{}", format_output(&result, *fmt));
            Ok(())
        }
        MigrateSubcommands::Validate {
            target,
            namespace,
            report,
        } => {
            let result = crate::migration::run_migrate_validate(target, namespace, report)?;
            println!("{}", format_output(&result, *fmt));
            Ok(())
        }
        MigrateSubcommands::Report {
            target,
            namespace,
            format: rpt_fmt,
            output,
        } => {
            let result = crate::migration::run_migrate_report(target, namespace, &rpt_fmt, output)?;
            println!("{}", format_output(&result, *fmt));
            Ok(())
        }
    }
}

async fn handle_config(
    cmd: command::ConfigSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::config::handle_config(cmd, global, fmt).await
}

async fn handle_doctor(
    aggressive: bool,
    report: Option<PathBuf>,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::doctor::handle_doctor(aggressive, report, fmt).await
}

async fn handle_discover(
    broadcast: String,
    port: u16,
    timeout: u64,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::discover::handle_discover(broadcast, port, timeout, fmt).await
}

async fn handle_governor(
    cmd: GovernorSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::governor::handle_governor(cmd, global, fmt).await
}

fn handle_completion(shell: String) -> Result<()> {
    use clap_complete::{generate, Shell};

    let mut cmd = Cli::command();
    let name = "primusdb";

    let shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "ps1" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        other => {
            let data = OutputData::Error(format!(
                "Unknown shell: {}. Supported: bash, zsh, fish, powershell, elvish",
                other
            ));
            println!("{}", format_output(&data, OutputFormat::Plain));
            return Ok(());
        }
    };

    generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

fn handle_version(verbose: bool) -> Result<()> {
    if verbose {
        println!(
            "PrimusDB v{} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_LICENSE")
        );
        println!("Build: {}", env!("CARGO_PKG_NAME"));
    } else {
        println!("{}", env!("CARGO_PKG_VERSION"));
    }
    Ok(())
}
