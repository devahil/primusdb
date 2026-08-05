/// Output formatting for PrimusDB CLI.
///
/// Supports multiple output formats for results and errors.
///
/// # Formats
/// - **Table**: Human-readable aligned columns
/// - **Json**: Machine-readable JSON
/// - **Csv**: Comma-separated values
/// - **Yaml**: YAML format
/// - **Plain**: Simple plain text
///
/// # Usage
/// ```ignore
/// use primusdb::cli::output::{format_output, OutputFormat, OutputData};
///
/// let data = OutputData::Table {
///     headers: vec!["Name", "Age"],
///     rows: vec![vec!["Alice".into(), "30".into()]],
/// };
/// let formatted = format_output(&data, OutputFormat::Table);
/// println!("{}", formatted);
/// ```
use serde::Serialize;
use std::str::FromStr;

/// Output format selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    /// Human-readable aligned columns
    Table,
    /// Machine-readable JSON
    Json,
    /// Comma-separated values
    Csv,
    /// YAML (currently falls back to JSON rendering)
    Yaml,
    /// Simple plain text
    Plain,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            "yaml" | "yml" => Ok(OutputFormat::Yaml),
            "plain" | "text" => Ok(OutputFormat::Plain),
            _ => Ok(OutputFormat::Table),
        }
    }
}

/// Structured data for output
#[derive(Debug, Clone, Serialize)]
pub enum OutputData {
    /// Table with headers and rows
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// Single value
    Value(String),
    /// Structured JSON value
    Json(serde_json::Value),
    /// List of strings
    List(Vec<String>),
    /// Key-value pairs
    Map(Vec<(String, String)>),
    /// A message
    Message(String),
    /// An error message
    Error(String),
}

/// Format output data into a string
pub fn format_output(data: &OutputData, format: OutputFormat) -> String {
    match format {
        OutputFormat::Table => format_table(data),
        OutputFormat::Json => format_json(data),
        OutputFormat::Csv => format_csv(data),
        OutputFormat::Yaml => format_yaml(data),
        OutputFormat::Plain => format_plain(data),
    }
}

fn format_table(data: &OutputData) -> String {
    match data {
        OutputData::Table { headers, rows } => {
            if rows.is_empty() {
                return "No results.".to_string();
            }
            // Calculate column widths
            let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
            for row in rows {
                for (i, cell) in row.iter().enumerate() {
                    if i < widths.len() {
                        widths[i] = widths[i].max(cell.len());
                    }
                }
            }

            let mut out = String::new();
            // Header
            for (i, h) in headers.iter().enumerate() {
                if i > 0 {
                    out.push_str("  ");
                }
                out.push_str(&format!("{:width$}", h, width = widths[i]));
            }
            out.push('\n');

            // Separator
            for w in &widths {
                out.push_str(&"-".repeat(*w));
                out.push_str("  ");
            }
            out.push('\n');

            // Rows
            for row in rows {
                for (i, cell) in row.iter().enumerate() {
                    if i > 0 {
                        out.push_str("  ");
                    }
                    out.push_str(&format!("{:width$}", cell, width = widths[i]));
                }
                out.push('\n');
            }
            out
        }
        OutputData::Message(msg) => msg.clone(),
        OutputData::Error(msg) => format!("Error: {}", msg),
        OutputData::Value(v) => v.clone(),
        OutputData::List(items) => items.join("\n"),
        OutputData::Map(pairs) => {
            let max_key = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            pairs
                .iter()
                .map(|(k, v)| format!("  {:width$}  {}", k, v, width = max_key))
                .collect::<Vec<_>>()
                .join("\n")
        }
        OutputData::Json(val) => serde_json::to_string_pretty(val).unwrap_or_default(),
    }
}

fn format_json(data: &OutputData) -> String {
    match data {
        OutputData::Table { headers, rows } => {
            let items: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| {
                    let mut obj = serde_json::Map::new();
                    for (i, h) in headers.iter().enumerate() {
                        if i < row.len() {
                            obj.insert(h.clone(), serde_json::Value::String(row[i].clone()));
                        }
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();
            serde_json::to_string_pretty(&items).unwrap_or_default()
        }
        OutputData::Json(val) => serde_json::to_string_pretty(val).unwrap_or_default(),
        OutputData::Message(msg) => {
            serde_json::to_string_pretty(&serde_json::json!({"message": msg})).unwrap_or_default()
        }
        OutputData::Error(msg) => {
            serde_json::to_string_pretty(&serde_json::json!({"error": msg})).unwrap_or_default()
        }
        OutputData::Value(v) => {
            serde_json::to_string_pretty(&serde_json::json!({"value": v})).unwrap_or_default()
        }
        OutputData::List(items) => serde_json::to_string_pretty(&items).unwrap_or_default(),
        OutputData::Map(pairs) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in pairs {
                obj.insert(k.clone(), serde_json::Value::String(v.clone()));
            }
            serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap_or_default()
        }
    }
}

fn format_csv(data: &OutputData) -> String {
    match data {
        OutputData::Table { headers, rows } => {
            let mut out = String::new();
            out.push_str(
                &headers
                    .iter()
                    .map(|h| escape_csv(h))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push('\n');
            for row in rows {
                out.push_str(
                    &row.iter()
                        .map(|c| escape_csv(c))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                out.push('\n');
            }
            out
        }
        _ => format_json(data),
    }
}

fn format_yaml(data: &OutputData) -> String {
    // serde_yaml is not a dependency; fall back to JSON
    format_json(data)
}

fn format_plain(data: &OutputData) -> String {
    match data {
        OutputData::Table { headers, rows } => {
            let mut out = String::new();
            for row in rows {
                let line: Vec<String> = row
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let h = headers
                            .get(i)
                            .map(|h| format!("{}: ", h))
                            .unwrap_or_default();
                        format!("{}{}", h, cell)
                    })
                    .collect();
                out.push_str(&line.join(", "));
                out.push('\n');
            }
            out
        }
        OutputData::Message(msg) => msg.clone(),
        OutputData::Error(msg) => format!("Error: {}", msg),
        OutputData::Value(v) => v.clone(),
        OutputData::List(items) => items.join("\n"),
        OutputData::Map(pairs) => pairs
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n"),
        OutputData::Json(val) => format!("{}", val),
    }
}

/// Quote a CSV field when it contains commas, quotes or newlines.
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Exit codes for CLI commands
#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    /// Command completed successfully
    Success = 0,
    /// Generic failure
    Error = 1,
    /// Invalid command-line arguments
    InvalidArgs = 2,
    /// Could not reach the server
    ConnectionFailure = 3,
    /// Authentication failed
    AuthFailure = 4,
    /// Query execution failed
    QueryError = 5,
    /// Requested resource was not found
    NotFound = 6,
    /// Operation not supported
    Unsupported = 7,
    /// Operation timed out
    Timeout = 8,
}

impl ExitCode {
    /// The numeric process exit status for this code.
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> OutputData {
        OutputData::Table {
            headers: vec!["Name".into(), "Age".into(), "City".into()],
            rows: vec![
                vec!["Alice".into(), "30".into(), "NYC".into()],
                vec!["Bob".into(), "25".into(), "SF".into()],
            ],
        }
    }

    #[test]
    fn test_format_table_basic() {
        let data = sample_table();
        let result = format_output(&data, OutputFormat::Table);
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
        assert!(result.contains("Name"));
        assert!(result.contains("---"));
    }

    #[test]
    fn test_format_json() {
        let data = sample_table();
        let result = format_output(&data, OutputFormat::Json);
        assert!(result.contains("Alice"));
        assert!(result.contains("Name"));
    }

    #[test]
    fn test_format_csv() {
        let data = sample_table();
        let result = format_output(&data, OutputFormat::Csv);
        assert!(result.contains("Alice,30,NYC"));
    }

    #[test]
    fn test_format_message() {
        let data = OutputData::Message("Hello, World!".into());
        let result = format_output(&data, OutputFormat::Table);
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_format_error() {
        let data = OutputData::Error("Something went wrong".into());
        let result = format_output(&data, OutputFormat::Table);
        assert_eq!(result, "Error: Something went wrong");
    }

    #[test]
    fn test_format_list() {
        let data = OutputData::List(vec!["a".into(), "b".into(), "c".into()]);
        let result = format_output(&data, OutputFormat::Plain);
        assert_eq!(result, "a\nb\nc");
    }

    #[test]
    fn test_format_map() {
        let data = OutputData::Map(vec![
            ("key1".into(), "val1".into()),
            ("key2".into(), "val2".into()),
        ]);
        let result = format_output(&data, OutputFormat::Table);
        assert!(result.contains("key1"));
        assert!(result.contains("val1"));
    }

    #[test]
    fn test_format_json_map() {
        let data = OutputData::Map(vec![("name".into(), "Alice".into())]);
        let result = format_output(&data, OutputFormat::Json);
        assert!(
            result.contains("{\n  \"name\": \"Alice\"\n}")
                || result.contains("\"name\": \"Alice\"")
        );
    }

    #[test]
    fn test_empty_table() {
        let data = OutputData::Table {
            headers: vec!["H".into()],
            rows: vec![],
        };
        let result = format_output(&data, OutputFormat::Table);
        assert_eq!(result, "No results.");
    }

    #[test]
    fn test_csv_escaping() {
        let data = OutputData::Table {
            headers: vec!["A".into()],
            rows: vec![vec!["hello, world".into()]],
        };
        let result = format_output(&data, OutputFormat::Csv);
        assert_eq!(result, "A\n\"hello, world\"\n");
    }

    #[test]
    fn test_exit_code_values() {
        assert_eq!(ExitCode::Success.code(), 0);
        assert_eq!(ExitCode::Error.code(), 1);
        assert_eq!(ExitCode::InvalidArgs.code(), 2);
        assert_eq!(ExitCode::QueryError.code(), 5);
        assert_eq!(ExitCode::Timeout.code(), 8);
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("csv").unwrap(), OutputFormat::Csv);
        assert_eq!(OutputFormat::from_str("yaml").unwrap(), OutputFormat::Yaml);
        assert_eq!(OutputFormat::from_str("yml").unwrap(), OutputFormat::Yaml);
        assert_eq!(
            OutputFormat::from_str("table").unwrap(),
            OutputFormat::Table
        );
        assert_eq!(
            OutputFormat::from_str("unknown").unwrap(),
            OutputFormat::Table
        );
    }
}
