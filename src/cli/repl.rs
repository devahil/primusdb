//! Interactive REPL shell for the PrimusDB CLI.
//!
//! `primusdb shell [url]` (and `primusdb connect [url]`) drop into a
//! console-in-console: every line is parsed as a regular `primusdb` CLI
//! invocation and dispatched through `run_cli` against the connected server,
//! with Tab completion, inline hints and a persistent command history.
//!
//! # REPL loop
//!
//! [`run`] builds a [`ReplState`] and a rustyline `Editor`, then loops
//! `readline → eval` until the user exits (see [`eval`]). Each line is:
//!
//! 1. Checked against the REPL-only commands (`help`/`?`, `history`,
//!    `clear`, `connect`, `use`, `exit`, `quit`, `disconnect`).
//! 2. Otherwise re-parsed through the **full CLI** via `Cli::try_parse_from`,
//!    so every regular `primusdb` subcommand works inside the shell.
//! 3. The parsed [`Cli`] is patched to default to the connected server
//!    (`state.server_url`) and the active database, then dispatched through
//!    [`run_cli`] — re-entering the normal command pipeline once per line.
//!
//! # Session state
//!
//! [`ReplState`] tracks the connected server URL, the active database and
//! the in-memory command history. The history is persisted to
//! `~/.config/primusdb/history` (or `./history` when `HOME` is unset) and
//! reloaded at the start of the next session.
//!
//! # Shell ergonomics
//!
//! [`ReplHelper`] provides rustyline Tab completion (subcommands, flags,
//! flag values, database names for `use`), inline hints for ambiguous
//! top-level commands, and a database list fetched from the server.

use std::path::PathBuf;

use clap::CommandFactory;
use clap::Parser;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::FileHistory;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

use crate::capabilities::ServerCapabilities;
use crate::cli::command::{Cli, Commands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::cli::run_cli;
use crate::Result;

const HISTORY_FILE: &str = "history";

/// Runtime state for the interactive shell.
pub struct ReplState {
    /// URL of the server the shell is currently connected to.
    pub server_url: String,
    /// Currently selected database, injected into `query`/`sql` lines.
    pub database: Option<String>,
    /// Commands typed during this session, used by the `history` command.
    pub history: Vec<String>,
    /// Capability snapshot of the connected server (version, node id, engines).
    pub metadata: Option<ServerCapabilities>,
}

impl Default for ReplState {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            database: None,
            history: Vec::new(),
            metadata: None,
        }
    }
}

impl ReplState {
    pub fn new(server_url: Option<String>) -> Self {
        Self {
            server_url: normalize_url(
                &server_url.unwrap_or_else(|| "http://localhost:8080".into()),
            ),
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the interactive shell loop until the user exits.
///
/// Runs the REPL on a dedicated OS thread with its own runtime. This works no
/// matter where the call comes from: building a runtime inside the caller's
/// tokio runtime would panic ("Cannot start a runtime from within a runtime"),
/// which happens on the main `#[tokio::main]` thread and when the shell
/// re-enters itself through `run_cli` (`shell`, `instance connect`).
pub fn run(state: ReplState) -> Result<()> {
    let handle = std::thread::Builder::new()
        .name("primusdb-repl".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(run_async(state))
        })
        .map_err(|e| {
            crate::Error::ValidationError(format!("Failed to start shell thread: {}", e))
        })?;

    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(crate::Error::ValidationError(
            "Shell thread panicked".to_string(),
        )),
    }
}

async fn run_async(mut state: ReplState) -> Result<()> {
    let history_path = history_file_path();

    let db_names = fetch_databases(&state.server_url).await;
    state.metadata = fetch_capabilities(&state.server_url).await;
    let mut helper = ReplHelper {
        db_names,
        tables: tables_from_capabilities(state.metadata.as_ref()),
        server: state.server_url.clone(),
        database: state.database.clone(),
    };
    let mut editor = Editor::<ReplHelper, FileHistory>::new().map_err(|e| {
        crate::Error::ValidationError(format!("Failed to initialize line editor: {}", e))
    })?;
    editor.set_helper(Some(helper.clone()));

    if editor.load_history(&history_path).is_err() {
        // No history yet — that's fine.
    }

    print_banner(&state);

    loop {
        let prompt = prompt(&state);
        match editor.readline(&prompt) {
            Ok(line) => {
                editor.add_history_entry(line.as_str()).ok();
                state.history.push(line.clone());
                if !eval(&mut state, &line).await {
                    break;
                }
                // Refresh completion when the server or active database changed.
                if helper.server != state.server_url || helper.database != state.database {
                    if helper.server != state.server_url {
                        state.metadata = fetch_capabilities(&state.server_url).await;
                    }
                    let db_names = fetch_databases(&state.server_url).await;
                    helper = ReplHelper {
                        db_names,
                        tables: tables_from_capabilities(state.metadata.as_ref()),
                        server: state.server_url.clone(),
                        database: state.database.clone(),
                    };
                    editor.set_helper(Some(helper.clone()));
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("(^C)");
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                let data = OutputData::Error(format!("Readline error: {}", e));
                println!("{}", format_output(&data, OutputFormat::Plain));
                break;
            }
        }
    }

    let _ = editor.save_history(&history_path);
    println!("Bye.");
    Ok(())
}

/// Print the startup banner with server metadata when available.
fn print_banner(state: &ReplState) {
    match state.metadata.as_ref() {
        Some(md) => {
            let database_count = md.engines.iter().map(|e| e.tables.len()).sum::<usize>();
            println!(
                "Connected to {} — PrimusDB v{} · node {} · instance {} · {} tables",
                state.server_url,
                md.server.version,
                md.server.node_id,
                md.server.instance_id,
                database_count
            );
            let engines: Vec<&str> = md.engines.iter().map(|e| e.storage_type.as_str()).collect();
            println!("Engines: {}", engines.join(", "));
        }
        None => println!(
            "Connected to {} (type 'help' for commands, 'exit' to quit)",
            state.server_url
        ),
    }
    println!("Type 'help' for commands, 'exit' to quit.");
}

/// Merge every engine's tables into a flat completion list.
fn tables_from_capabilities(md: Option<&ServerCapabilities>) -> Vec<String> {
    match md {
        Some(md) => md
            .engines
            .iter()
            .flat_map(|e| e.tables.iter().cloned())
            .collect(),
        None => Vec::new(),
    }
}

/// Resolve the persistent history file under `~/.config/primusdb`.
fn history_file_path() -> PathBuf {
    let dir = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config").join("primusdb"))
        .unwrap_or_else(|_| PathBuf::from("."));
    dir.join(HISTORY_FILE)
}

/// Render the shell prompt, showing the host and active database.
fn prompt(state: &ReplState) -> String {
    let host = state
        .server_url
        .replace("http://", "")
        .replace("https://", "");
    match &state.database {
        Some(db) => format!("primusdb@{} [{}]> ", host, db),
        None => format!("primusdb@{}> ", host),
    }
}

/// Ensure a server argument carries an `http://` / `https://` scheme.
fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{}", url)
    }
}

// ---------------------------------------------------------------------------
// Command evaluation
// ---------------------------------------------------------------------------

/// Evaluate a single REPL line. Returns `false` when the shell must exit.
async fn eval(state: &mut ReplState, line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }

    let tokens: Vec<String> = shlex::split(trimmed)
        .unwrap_or_else(|| trimmed.split_whitespace().map(|s| s.to_string()).collect());
    if tokens.is_empty() {
        return true;
    }

    let first = tokens[0].as_str();
    match first {
        "exit" | "quit" => return false,
        "disconnect" => return false,
        "clear" => {
            print!("\x1b[2J\x1b[H");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            return true;
        }
        "help" | "?" => {
            print_help();
            return true;
        }
        "history" => {
            for (i, h) in state.history.iter().enumerate() {
                println!("{:>4}  {}", i + 1, h);
            }
            return true;
        }
        "connect" => {
            match tokens.get(1) {
                Some(url) => {
                    let url = normalize_url(url);
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .map_err(|e| crate::Error::NetworkError(e.to_string()));
                    let ok = match client {
                        Ok(client) => match client.get(format!("{}/health", url)).send().await {
                            Ok(resp) => resp.status().is_success(),
                            Err(_) => false,
                        },
                        Err(_) => false,
                    };
                    if ok {
                        state.server_url = url.clone();
                        state.database = None;
                        println!("Connected to {}", url);
                    } else {
                        let data = OutputData::Error(format!("Connection failed: {}", url));
                        println!("{}", format_output(&data, OutputFormat::Plain));
                    }
                }
                None => {
                    let data = OutputData::Error("Usage: connect [http://host:port]".into());
                    println!("{}", format_output(&data, OutputFormat::Plain));
                }
            }
            return true;
        }
        "use" => {
            match tokens.get(1) {
                Some(db) if db != "none" && db != "-" => {
                    state.database = Some(db.clone());
                    println!("Switched to database '{}'", db);
                }
                _ => {
                    state.database = None;
                    println!("No active database");
                }
            }
            return true;
        }
        _ => {}
    }

    // Otherwise, treat the line as a regular CLI invocation.
    let has_server = tokens
        .iter()
        .any(|t| t == "--server" || t == "--server-url");

    let mut argv = vec!["primusdb".to_string()];
    argv.extend(tokens);

    match Cli::try_parse_from(&argv) {
        Ok(mut cli) => {
            if !has_server {
                cli.global.server_url = state.server_url.clone();
            }
            if let Some(db) = state.database.clone() {
                if !has_db_flag(&cli.command) {
                    inject_database(&mut cli.command, &db);
                }
            }
            if let Err(e) = run_cli(cli).await {
                let data = OutputData::Error(e.to_string());
                println!("{}", format_output(&data, OutputFormat::Plain));
            }
        }
        Err(e) => {
            let data = OutputData::Error(format!("{}", e.render()));
            println!("{}", format_output(&data, OutputFormat::Plain));
            println!("Type 'help' for available commands.");
        }
    }
    true
}

fn has_db_flag(cmd: &Commands) -> bool {
    match cmd {
        Commands::Query { database, .. } | Commands::Sql { database, .. } => database.is_some(),
        _ => false,
    }
}

fn inject_database(cmd: &mut Commands, db: &str) {
    match cmd {
        Commands::Query { database, .. } | Commands::Sql { database, .. } => {
            database.get_or_insert_with(|| db.to_string());
        }
        _ => {}
    }
}

/// Print the in-shell help text describing REPL and CLI commands.
fn print_help() {
    println!("PrimusDB interactive shell");
    println!("========================================");
    println!("REPL commands:");
    println!("  connect [url]     Connect to / switch server (e.g. connect 192.168.1.5:8080)");
    println!("  use <db>|none     Set the active database for query/sql");
    println!("  help, ?           Show this help");
    println!("  history           Show command history");
    println!("  clear             Clear the screen");
    println!("  exit, quit, ^D    Leave the shell");
    println!();
    println!("Every other line is a normal `primusdb` command against the connected");
    println!("server. Common examples:");
    println!("  health");
    println!("  status");
    println!("  db list");
    println!("  db create mydb --engine relational");
    println!("  query \"SELECT * FROM users\"");
    println!("  ts ingest sensor_readings --value 23.5 --tags 'sensor=a'");
    println!("  kv put mydb doc1 '{{\"name\": \"ada\"}}'");
    println!("  config show");
    println!("  cluster status");
    println!("Use --format json for machine-readable output, Tab for completion.");
}

// ---------------------------------------------------------------------------
// Completion / hints (rustyline)
// ---------------------------------------------------------------------------

/// rustyline helper providing Tab completion and inline hints.
#[derive(Clone)]
struct ReplHelper {
    db_names: Vec<String>,
    /// Table/collection names across all engines of the connected server.
    tables: Vec<String>,
    /// Server + database the completion lists were fetched for.
    server: String,
    database: Option<String>,
}

impl Helper for ReplHelper {}
impl Highlighter for ReplHelper {}
impl Validator for ReplHelper {}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let before = &line[..pos];
        let word_start = before
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &before[word_start..];
        let path: Vec<&str> = before[..word_start].split_whitespace().collect();

        let mut candidates: Vec<String> = Vec::new();

        if path.is_empty() {
            for (name, _) in top_level_commands() {
                if name.starts_with(word) && !name.is_empty() {
                    candidates.push(name);
                }
            }
            for flag in [
                "--server",
                "--format",
                "--timeout",
                "--output",
                "--help",
                "--version",
            ] {
                if flag.starts_with(word) {
                    candidates.push(flag.to_string());
                }
            }
        } else {
            let mut cmd = Cli::command();
            let mut remaining = path.as_slice();
            let mut matched = true;
            while let Some(head) = remaining.first() {
                match cmd.find_subcommand_mut(head) {
                    Some(sub) => {
                        cmd = sub.clone();
                        remaining = &remaining[1..];
                    }
                    None => {
                        matched = false;
                        break;
                    }
                }
            }
            if matched {
                for sub in cmd.get_subcommands() {
                    let name = sub.get_name();
                    if name.starts_with(word) && !name.is_empty() {
                        candidates.push(name.to_string());
                    }
                }
                for arg in cmd.get_arguments() {
                    if let Some(long) = arg.get_long() {
                        let flag = format!("--{}", long);
                        if flag.starts_with(word) {
                            candidates.push(flag);
                        }
                    }
                    if let Some(short) = arg.get_short() {
                        let flag = format!("-{}", short);
                        if flag.starts_with(word) {
                            candidates.push(flag);
                        }
                    }
                }
            }
        }

        // Value completions for common flags.
        if let Some(prev) = path.last() {
            let values: &[&str] = match *prev {
                "--format" | "-f" => &["table", "json", "csv", "yaml", "plain"],
                "--engine" | "-e" | "--storage" | "--storage-type" | "--type" => &[
                    "relational",
                    "document",
                    "keyvalue",
                    "columnar",
                    "vector",
                    "timeseries",
                ],
                _ => &[],
            };
            for v in values {
                if v.starts_with(word) {
                    candidates.push((*v).to_string());
                }
            }
        }

        // `use <db>` completes from the server's database list.
        if path.first().map(|s| *s == "use").unwrap_or(false) {
            for db in &self.db_names {
                if db.starts_with(word) {
                    candidates.push(db.clone());
                }
            }
        }

        // Table names complete for commands that reference tables.
        let table_commands = [
            "query",
            "sql",
            "search",
            "ts",
            "vector",
            "analyze",
            "anomalies",
            "info",
        ];
        if let Some(first) = path.first() {
            if table_commands.contains(first) && !word.is_empty() {
                for t in &self.tables {
                    if t.starts_with(word) {
                        candidates.push(t.clone());
                    }
                }
            }
        }

        // Deduplicate preserving order.
        let mut seen = std::collections::HashSet::new();
        let pairs: Vec<Pair> = candidates
            .into_iter()
            .filter(|c| seen.insert(c.clone()))
            .map(|c| Pair {
                display: c.clone(),
                replacement: c,
            })
            .collect();
        Ok((word_start, pairs))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if pos != line.len() {
            return None;
        }
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            return None;
        }
        let mut matches: Vec<(String, String)> = top_level_commands()
            .into_iter()
            .filter(|(name, _)| name.starts_with(trimmed))
            .collect();
        if matches.len() == 1 {
            let (name, about) = matches.remove(0);
            if name.len() > trimmed.len() {
                let suffix = &name[trimmed.len()..];
                return Some(format!("{} — {}", suffix, about));
            }
        }
        None
    }
}

/// Build the list of completable top-level commands (CLI + REPL-only).
fn top_level_commands() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for sub in Cli::command().get_subcommands() {
        out.push((
            sub.get_name().to_string(),
            sub.get_about().map(|a| a.to_string()).unwrap_or_default(),
        ));
    }
    for (name, about) in [
        ("connect", "connect to / switch server"),
        ("disconnect", "leave the interactive shell"),
        ("use", "switch active database"),
        ("help", "show help"),
        ("history", "show command history"),
        ("clear", "clear the screen"),
        ("exit", "leave the shell"),
        ("quit", "leave the shell"),
    ] {
        out.push((name.to_string(), about.to_string()));
    }
    out
}

// ---------------------------------------------------------------------------
// Server introspection
// ---------------------------------------------------------------------------

/// Fetch the server's database names for `use <db>` completion.
async fn fetch_databases(url: &str) -> Vec<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    match client.get(format!("{}/api/v1/databases", url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let data = json.get("data").unwrap_or(&json);
                if let Some(arr) = data.as_array() {
                    return arr
                        .iter()
                        .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
                        .map(|s| s.to_string())
                        .collect();
                }
            }
        }
        _ => {}
    }
    Vec::new()
}

/// Fetch the server capability snapshot (version, node id, engines, tables).
async fn fetch_capabilities(url: &str) -> Option<ServerCapabilities> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => return None,
    };
    match client
        .get(format!("{}/api/v1/capabilities", url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                return json
                    .get("data")
                    .cloned()
                    .and_then(|data| serde_json::from_value::<ServerCapabilities>(data).ok());
            }
            None
        }
        _ => None,
    }
}
