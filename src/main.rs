/// PrimusDB - Unified Command-Line Interface
///
/// This is the primary entry point for all PrimusDB operations.
/// Use `primusdb --help` to see available commands.
use primusdb::cli;

#[tokio::main]
async fn main() -> primusdb::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Dispatch to unified CLI handler
    cli::run().await
}
