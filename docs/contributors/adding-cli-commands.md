# Adding CLI Commands

This guide explains how to add new CLI commands to the PrimusDB unified CLI.
The CLI uses [clap](https://docs.rs/clap/latest/clap/) with derive macros for
argument parsing.

## CLI Architecture

```
src/cli/
├── command.rs          # Step 1: Define clap types (enum variants + structs)
├── cmd/                # Step 2: Command handler implementations
│   ├── mod.rs          # Step 4: Register new handler module
│   ├── db.rs           # Existing handlers (patterns to follow)
│   ├── query.rs
│   ├── server.rs
│   └── ...
├── mod.rs              # Step 3: Wire match arm + handler function
└── output.rs           # Output formatting utilities
```

The flow is:

```
User Input
    │
    ▼
command.rs  ◄── Clap parses args into typed enums/structs
    │
    ▼
mod.rs      ◄── Match on Commands variant, call handler fn
    │
    ▼
cmd/*.rs    ◄── Handler fn performs logic, returns Result<()>
    │
    ▼
output.rs   ◄── Format and print output
```

## Step-by-Step Guide

### Step 1: Define Types in `src/cli/command.rs`

Add a new variant to the `Commands` enum and define any associated
subcommand types.

For a simple command with no subcommands, add a variant directly:

```rust
// In src/cli/command.rs — add to the Commands enum

/// Say hello to PrimusDB
Hello {
    /// Name to greet
    #[arg(required = true)]
    name: String,
    /// Number of times to repeat
    #[arg(short, long, default_value = "1")]
    count: u32,
    /// Whether to be formal
    #[arg(long)]
    formal: bool,
},
```

For a command with subcommands, define a subcommand enum and add it
to `Commands`:

```rust
// In src/cli/command.rs — subcommand enum

#[derive(Subcommand)]
pub enum HelloSubcommands {
    /// Greet a user
    Greet {
        #[arg(required = true)]
        name: String,
    },
    /// List available greetings
    List {
        #[arg(long)]
        verbose: bool,
    },
}

// In the Commands enum:
/// Hello world commands
#[command(subcommand)]
Hello(HelloSubcommands),
```

### Step 2: Create Handler in `src/cli/cmd/your_module.rs`

Create a new file in `src/cli/cmd/` with the handler function.
The handler receives the parsed arguments and an `OutputFormat`,
and returns `Result<()>`.

```rust
// src/cli/cmd/hello.rs

use crate::cli::command::GlobalArgs;
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

pub async fn handle_hello(
    name: String,
    count: u32,
    formal: bool,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let greeting = if formal {
        format!("Good day, {}!", name)
    } else {
        format!("Hello, {}!", name)
    };

    let lines: Vec<String> = (0..count).map(|_| greeting.clone()).collect();

    let data = OutputData::List(lines);
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

/// Handler for hello subcommands
pub async fn handle_hello_sub(
    cmd: crate::cli::command::HelloSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        crate::cli::command::HelloSubcommands::Greet { name } => {
            let data = OutputData::Message(format!("Hello, {}!", name));
            println!("{}", format_output(&data, *fmt));
        }
        crate::cli::command::HelloSubcommands::List { verbose } => {
            let greetings = if verbose {
                vec!["Hello", "Hi", "Hey", "Howdy"]
            } else {
                vec!["Hello", "Hi"]
            };
            let data = OutputData::List(greetings.iter().map(|s| s.to_string()).collect());
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}
```

**Patterns to follow** (from existing handlers):

- Handlers are `pub async fn` returning `crate::Result<()>`
- Use `OutputData` variants (`Table`, `List`, `Message`, `Map`, etc.)
  for structured output
- Call `format_output(&data, *fmt)` to format according to user preference
- Accept `&GlobalArgs` for global flags (server URL, timeout, output file)
- Accept `&OutputFormat` for the user's chosen output format

### Step 3: Wire in `src/cli/mod.rs`

Two things are needed: a match arm in the `run()` function and a
handler function.

Add the match arm in `run()`:

```rust
// In src/cli/mod.rs — add to the match block in run()

Commands::Hello { name, count, formal } => {
    handle_hello(name, count, formal, &cli.global, &fmt).await
}

// For subcommand variants:
Commands::Hello(cmd) => handle_hello_sub(cmd, &cli.global, &fmt).await,
```

Add the handler function declaration (if it's a simple handler, not
delegating to a cmd module):

```rust
// In src/cli/mod.rs — add handler function

async fn handle_hello(
    name: String,
    count: u32,
    formal: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::hello::handle_hello(name, count, formal, global, fmt).await
}
```

### Step 4: Add Module to `src/cli/cmd/mod.rs`

```rust
// In src/cli/cmd/mod.rs — add to the module declarations

pub mod hello;
```

### Step 5: Test

Build and verify your command compiles:

```bash
cargo build --workspace
```

Run the command:

```bash
cargo run -- hello Alice
# Output: Hello, Alice!

cargo run -- hello --count 3 --formal Bob
# Output: Good day, Bob!
#         Good day, Bob!
#         Good day, Bob!

cargo run -- hello --format json Alice
# Output: ["Hello, Alice!"]
```

Run existing tests to make sure nothing is broken:

```bash
cargo test --workspace
```

Add tests for your handler. Tests can go in-file (`#[cfg(test)]`)
and test the handler logic directly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::output::OutputFormat;

    #[tokio::test]
    async fn test_handle_hello_basic() {
        let global = GlobalArgs {
            server_url: "http://localhost:8080".into(),
            format: "plain".into(),
            timeout: 30000,
            output: None,
        };
        let result = handle_hello(
            "World".into(),
            1,
            false,
            &global,
            &OutputFormat::Plain,
        ).await;
        assert!(result.is_ok());
    }
}
```

## Complete Example

Here is a complete, minimal "hello" command added across all files:

### `src/cli/command.rs` — Add variant:

```rust
/// Say hello
Hello {
    /// Who to greet
    name: String,
    /// Repeat count
    #[arg(short, long, default_value = "1")]
    count: u32,
},
```

### `src/cli/cmd/hello.rs` — Create handler:

```rust
use crate::cli::command::GlobalArgs;
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

pub async fn handle_hello(
    name: String,
    count: u32,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let lines = (0..count)
        .map(|_| format!("Hello, {}!", name))
        .collect::<Vec<_>>();
    let data = OutputData::List(lines);
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
```

### `src/cli/mod.rs` — Add match arm and handler:

In `run()`:
```rust
Commands::Hello { name, count } => {
    handle_hello(name, count, &cli.global, &fmt).await
}
```

Handler:
```rust
async fn handle_hello(
    name: String,
    count: u32,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    cmd::hello::handle_hello(name, count, global, fmt).await
}
```

### `src/cli/cmd/mod.rs` — Add module:

```rust
pub mod hello;
```

## Existing Commands as Reference

| Command | File | Pattern |
|---------|------|---------|
| `db` | `cmd/db.rs` | Subcommand enum + handler per sub-variant |
| `query` | `cmd/query.rs` | Simple args + handler |
| `server` | `cmd/server.rs` | Subcommand enum with complex options |
| `auth` | `cmd/auth.rs` | Multiple subcommands in one file |
| `ai` | `cmd/ai.rs` | Subcommand + global args usage |
| `backup` | `cmd/backup.rs` | Multiple subcommands with shared state |
| `version` | `mod.rs` (inline) | Simple handler, no separate file |

## Best Practices

1. **Use `OutputData` variants** instead of printing directly. This ensures
   all output formats (table, json, csv, yaml, plain) work correctly.

2. **Keep handlers focused** — each handler should do one thing. If the
   command is complex, split into helper functions.

3. **Reuse existing patterns** — look at `cmd/db.rs` for standard CRUD
   patterns, `cmd/cluster.rs` for distributed operations, and `cmd/ai.rs`
   for computation-heavy commands.

4. **Add tests** — each handler should have at least one unit test that
   verifies its basic operation.

5. **Document with clap attributes** — use `///` comments on enum variants
   and struct fields — clap uses these as help text.

6. **Error handling** — return `crate::Result<()>` with specific error
   variants. The `?` operator converts errors automatically via `From`
   implementations.
