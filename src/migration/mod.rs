//! Migration framework for importing data from external databases into PrimusDB.
//!
//! The migration workflow follows these stages:
//!
//! ```text
//! +-------------+
//! | Source DB   |
//! +------+------+
//!        | inspect
//!        v
//! +-------------+
//! | Plan        |
//! +------+------+
//!        | map
//!        v
//! +-------------+
//! | PrimusDB    |
//! +------+------+
//!        | validate
//!        v
//! +-------------+
//! | Report      |
//! +-------------+
//! ```
//!
//! ## Supported Sources
//!
//! The three driver-based sources are gated behind optional Cargo features
//! (`mysql-source`, `postgres-source`, `mongo-source`):
//!
//! - **MySQL** — requires the `mysql` crate (v28) behind `mysql-source`
//! - **PostgreSQL** — requires the `tokio-postgres` crate (v0.7) behind
//!   `postgres-source`
//! - **MongoDB** — requires the `mongodb` crate (v3) behind `mongo-source`
//! - **CouchDB** — uses `reqwest` for the CouchDB REST API (no feature gate)
//!
//! ## Writers
//!
//! All writers connect to a running PrimusDB server via the REST API
//! using the `reqwest` crate. Supported target engines: relational,
//! columnar, document, keyvalue, vector.

pub mod mapping;
pub mod plan;
pub mod report;
pub mod source;
pub mod target;
pub mod validate;

pub mod sources;
pub mod writers;

use std::path::PathBuf;

use crate::cli::output::OutputData;
use crate::Result;

use self::plan::{generate_plan, render_plan, MigrationMode, MigrationPlan};
use self::report::MigrationReport;
use self::target::{DataBatch, ObjectMapping};
use self::validate::validate_import;

// ---------------------------------------------------------------------------
// CLI entry points (called from src/cli/mod.rs)
// ---------------------------------------------------------------------------

/// CLI entry point: generate a migration plan.
pub fn run_migrate_plan(
    source: String,
    url: String,
    target: Option<String>,
    namespace: Option<String>,
    mapping: Option<PathBuf>,
    mode: String,
    output: Option<PathBuf>,
) -> Result<OutputData> {
    let target = target.unwrap_or_else(|| "http://localhost:8080".to_string());
    let namespace = namespace.unwrap_or_else(|| "default".to_string());

    let rendered = match parse_mode(&mode) {
        Ok(mode_enum) => match sources::create_source(&source, &url) {
            Ok(source_obj) => match source_obj.inspect_schema() {
                Ok(schema) => {
                    let mapping_cfg = mapping.as_ref().and_then(|path| {
                        std::fs::read_to_string(path)
                            .ok()
                            .and_then(|content| mapping::parse_mapping(&content).ok())
                    });

                    let mut plan = generate_plan(&schema, mapping_cfg.as_ref(), &mode_enum);
                    plan.source_type = source.clone();
                    plan.source_url = url.clone();
                    plan.target_url = target;
                    plan.namespace = namespace;

                    let text = render_plan(&plan);

                    if let Some(path) = output {
                        match std::fs::write(&path, &text) {
                            Ok(_) => {
                                format!("Migration plan written to {}", path.display())
                            }
                            Err(e) => {
                                format!("Migration plan generated but write failed: {}", e)
                            }
                        }
                    } else {
                        text
                    }
                }
                Err(e) => format!("Schema inspection failed: {}", e),
            },
            Err(e) => format!("Source connection failed: {}", e),
        },
        Err(e) => format!("Invalid mode: {}", e),
    };

    Ok(OutputData::Message(rendered))
}

/// Configuration for a migration import operation.
pub struct MigrateImportConfig {
    /// Source database type (mysql, postgres, mongodb, couchdb).
    pub source: String,
    /// Source database connection URL.
    pub url: String,
    /// Target PrimusDB server URL.
    pub target: Option<String>,
    /// Target namespace.
    pub namespace: Option<String>,
    /// Path to a TOML mapping configuration file.
    pub mapping: Option<PathBuf>,
    /// Migration mode (dry-run, schema-only, data-only, full).
    pub mode: String,
    /// Number of rows per batch.
    pub batch_size: u64,
    /// Maximum number of rows to import.
    pub limit: Option<u64>,
    /// Only include objects matching this glob pattern.
    pub include: Option<String>,
    /// Exclude objects matching this glob pattern.
    pub exclude: Option<String>,
    /// Overwrite existing data in target.
    pub overwrite: bool,
    /// Resume a partially-completed migration.
    pub resume: bool,
}

/// CLI entry point: import data from a source database.
pub fn run_migrate_import(cfg: MigrateImportConfig) -> Result<OutputData> {
    let target = cfg
        .target
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    let namespace = cfg.namespace.unwrap_or_else(|| "default".to_string());

    let mode_enum = match parse_mode(&cfg.mode) {
        Ok(m) => m,
        Err(e) => return Ok(OutputData::Error(format!("{}", e))),
    };

    let source_obj = match sources::create_source(&cfg.source, &cfg.url) {
        Ok(s) => s,
        Err(e) => return Ok(OutputData::Error(format!("{}", e))),
    };

    let schema = match source_obj.inspect_schema() {
        Ok(s) => s,
        Err(e) => return Ok(OutputData::Error(format!("{}", e))),
    };

    let mapping_cfg = match cfg.mapping {
        Some(ref path) => {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => return Ok(OutputData::Error(format!("{}", e))),
            };
            match mapping::parse_mapping(&content) {
                Ok(c) => Some(c),
                Err(e) => return Ok(OutputData::Error(format!("{}", e))),
            }
        }
        None => None,
    };

    let mut plan = generate_plan(&schema, mapping_cfg.as_ref(), &mode_enum);
    plan.source_type = cfg.source;
    plan.source_url = cfg.url;
    plan.target_url = target;
    plan.namespace = namespace;
    plan.batch_size = cfg.batch_size;

    if !cfg.overwrite {
        plan.warnings.push("Overwrite not set — existing data will be preserved. Use --overwrite to allow replacement.".to_string());
    }

    if cfg.resume {
        plan.warnings.push(
            "Resume mode enabled — skipping target creation. Target tables must already exist."
                .to_string(),
        );
    }

    if matches!(mode_enum, MigrationMode::DryRun) {
        return Ok(OutputData::Message(render_plan(&plan)));
    }

    // Apply include/exclude filters
    let filtered_objects: Vec<&ObjectMapping> = plan
        .objects
        .iter()
        .filter(|obj| {
            if let Some(ref include) = cfg.include {
                if !obj.target.contains(include) && !obj.source.contains(include) {
                    return false;
                }
            }
            if let Some(ref exclude) = cfg.exclude {
                if obj.target.contains(exclude) || obj.source.contains(exclude) {
                    return false;
                }
            }
            true
        })
        .collect();

    let mut report = MigrationReport {
        source_type: plan.source_type.clone(),
        source_url_masked: MigrationReport::mask_url(&plan.source_url),
        target_url: plan.target_url.clone(),
        namespace: plan.namespace.clone(),
        mode: mode_enum.to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        duration_ms: 0,
        objects_total: filtered_objects.len() as u64,
        objects_imported: 0,
        rows_total: plan.estimated_rows,
        rows_imported: 0,
        errors: vec![],
        warnings: plan.warnings.clone(),
        validation: None,
    };

    for obj_mapping in filtered_objects {
        let writer =
            match writers::create_writer(&obj_mapping.engine, &plan.target_url, &plan.namespace) {
                Ok(w) => w,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to create writer: {}", e));
                    continue;
                }
            };

        if !matches!(mode_enum, MigrationMode::DataOnly) && !cfg.resume {
            match writer.create_target(obj_mapping) {
                Ok(()) => report.objects_imported += 1,
                Err(e) => {
                    report.errors.push(format!(
                        "Failed to create target '{}': {}",
                        obj_mapping.target, e
                    ));
                    continue;
                }
            }
        }

        if !matches!(mode_enum, MigrationMode::SchemaOnly) {
            let source_object = schema
                .databases
                .iter()
                .flat_map(|db| db.objects.iter())
                .find(|o| {
                    let qualified = format!("{}.{}", db_name(o, &schema), o.name);
                    qualified == obj_mapping.source || o.name == obj_mapping.source
                })
                .cloned();

            let source_object = match source_object {
                Some(obj) => obj,
                None => {
                    report
                        .errors
                        .push(format!("Source object '{}' not found", obj_mapping.source));
                    continue;
                }
            };

            let row_stream = match source_obj.stream_rows(&source_object) {
                Ok(s) => s,
                Err(e) => {
                    report.errors.push(format!("Failed to stream rows: {}", e));
                    continue;
                }
            };

            let mut rows: Vec<Vec<serde_json::Value>> = row_stream.rows;
            if let Some(limit) = cfg.limit {
                if (report.rows_imported + rows.len() as u64) > limit {
                    let remaining = limit.saturating_sub(report.rows_imported) as usize;
                    rows.truncate(remaining);
                }
            }

            for chunk in rows.chunks(cfg.batch_size as usize) {
                let batch = DataBatch {
                    target: obj_mapping.target.clone(),
                    engine: obj_mapping.engine.clone(),
                    columns: row_stream.columns.clone(),
                    rows: chunk.to_vec(),
                };
                match writer.write_batch(batch) {
                    Ok(result) => {
                        report.rows_imported += result.rows_written;
                        report.errors.extend(result.errors);
                    }
                    Err(e) => {
                        report.errors.push(format!("Write batch failed: {}", e));
                    }
                }
            }
        }
    }

    if report.errors.is_empty() {
        match validate_import(&plan.target_url, &plan.namespace, &plan) {
            Ok(vr) => report.validation = Some(vr),
            Err(e) => report.warnings.push(format!("Validation skipped: {}", e)),
        }
    }

    Ok(OutputData::Message(report.render_markdown()))
}

/// CLI entry point: validate a completed migration.
pub fn run_migrate_validate(
    target: Option<String>,
    namespace: Option<String>,
    report_path: Option<PathBuf>,
) -> Result<OutputData> {
    let target = target.unwrap_or_else(|| "http://localhost:8080".to_string());
    let namespace = namespace.unwrap_or_else(|| "default".to_string());

    let output = if let Some(path) = report_path {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => return Ok(OutputData::Error(format!("{}", e))),
        };
        let plan: MigrationPlan = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(e) => return Ok(OutputData::Error(format!("{}", e))),
        };
        match validate_import(&target, &namespace, &plan) {
            Ok(vr) => {
                let mut msg = format!(
                    "Validation Report:\n  Objects checked: {}\n  Rows matched: {}\n  Checksums matched: {}",
                    vr.objects_checked, vr.rows_matched, vr.checksums_matched
                );
                if vr.mismatches.is_empty() {
                    msg.push_str("\n  Result: All checks passed");
                } else {
                    msg.push_str("\n  Mismatches:");
                    for m in &vr.mismatches {
                        msg.push_str(&format!("\n    - {}", m));
                    }
                }
                OutputData::Message(msg)
            }
            Err(e) => OutputData::Error(format!("Validation failed: {}", e)),
        }
    } else {
        OutputData::Message(
            "No migration plan file provided. Use --report to specify a plan file.".to_string(),
        )
    };

    Ok(output)
}

/// CLI entry point: generate a migration report.
pub fn run_migrate_report(
    target: Option<String>,
    namespace: Option<String>,
    format: &str,
    output: Option<PathBuf>,
) -> Result<OutputData> {
    let target = target.unwrap_or_else(|| "http://localhost:8080".to_string());
    let namespace = namespace.unwrap_or_else(|| "default".to_string());

    let report = MigrationReport {
        source_type: "unknown".into(),
        source_url_masked: "unknown".into(),
        target_url: target,
        namespace,
        mode: "unknown".into(),
        started_at: chrono::Utc::now().to_rfc3339(),
        duration_ms: 0,
        objects_total: 0,
        objects_imported: 0,
        rows_total: 0,
        rows_imported: 0,
        errors: vec![],
        warnings: vec!["No migration data available — run a migration first.".into()],
        validation: None,
    };

    let rendered = match format {
        "json" => serde_json::to_string_pretty(&report).unwrap_or_default(),
        _ => report.render_markdown(),
    };

    if let Some(path) = output {
        std::fs::write(&path, &rendered).map_err(crate::Error::IOError)?;
        Ok(OutputData::Message(format!(
            "Report written to {}",
            path.display()
        )))
    } else {
        Ok(OutputData::Message(rendered))
    }
}

/// Parse a mode string into a [`MigrationMode`].
fn parse_mode(s: &str) -> Result<MigrationMode> {
    match s {
        "copy" => Ok(MigrationMode::Copy),
        "schema-only" | "schema_only" | "schemaonly" => Ok(MigrationMode::SchemaOnly),
        "data-only" | "data_only" | "dataonly" => Ok(MigrationMode::DataOnly),
        "dry-run" | "dry_run" | "dryrun" => Ok(MigrationMode::DryRun),
        other => Err(crate::Error::InvalidRequest(format!(
            "Unknown migration mode '{}'. Valid modes: copy, schema-only, data-only, dry-run",
            other
        ))),
    }
}

fn db_name<'a>(obj: &'a source::SourceObject, schema: &'a source::SourceSchema) -> &'a str {
    for db in &schema.databases {
        if db.objects.iter().any(|o| std::ptr::eq(o, obj)) {
            return &db.name;
        }
    }
    ""
}
