use crate::cli::command::{GlobalArgs, GraphSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

pub async fn handle_graph(
    cmd: GraphSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let feature = match &cmd {
        GraphSubcommands::Nodes { .. } => "graph nodes",
        GraphSubcommands::Edges { .. } => "graph edges",
        GraphSubcommands::Query { .. } => "graph query",
        GraphSubcommands::Traverse { .. } => "graph traverse",
    };
    let data = OutputData::Message(format!(
        "{} is not yet available via CLI. \
         Use the PrimusDB SQL interface (`primusdb query`) to query graph data, \
         or use the TUI Graph section.",
        feature
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
