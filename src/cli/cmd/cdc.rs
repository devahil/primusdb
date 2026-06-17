use crate::cli::command::{CdcSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

pub async fn handle_cdc(
    cmd: CdcSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let feature = match &cmd {
        CdcSubcommands::Status { .. } => "cdc status",
        CdcSubcommands::Stream { .. } => "cdc stream",
        CdcSubcommands::Subscribe { .. } => "cdc subscribe",
        CdcSubcommands::Offsets { .. } => "cdc offsets",
    };
    let data = OutputData::Message(format!(
        "{} is not yet available via CLI. \
         CDC is functional at the library level (see docs/features/cdc.md) \
         but CLI bindings have not been implemented yet.",
        feature
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
