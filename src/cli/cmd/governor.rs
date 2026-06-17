use crate::cli::command::{GlobalArgs, GovernorSubcommands};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::governor::{
    EnforcementAction, ExecutionLimits, GovernorConfig,
};
use crate::governor::engine::GovernorEngine;
use crate::Result;

pub async fn handle_governor(
    cmd: GovernorSubcommands,
    _global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let engine = GovernorEngine::new(GovernorConfig::default());
    drop(engine);
    match cmd {
        GovernorSubcommands::Status => cmd_status(fmt).await,
        GovernorSubcommands::Policies { name } => cmd_policies(name, fmt).await,
        GovernorSubcommands::Inspect { execution_id } => cmd_inspect(execution_id, fmt).await,
        GovernorSubcommands::Metrics { watch, interval } => cmd_metrics(watch, interval, fmt).await,
        GovernorSubcommands::Violations {
            last,
            workload,
            limit,
        } => cmd_violations(last, workload, limit, fmt).await,
        GovernorSubcommands::Set {
            name,
            max_memory_mb,
            max_execution_steps,
            max_cpu_time_ms,
            max_query_complexity,
            max_join_count,
            max_sort_rows,
            max_pipeline_depth,
            max_pipeline_stages,
            max_ffi_calls,
            max_ffi_memory_mb,
            max_ffi_time_ms,
            max_training_iterations,
            max_prediction_batch_size,
            max_embedding_batch_size,
            max_vector_candidates,
            max_vector_expansions,
            max_graph_depth,
            max_graph_nodes,
            max_graph_edges,
            max_import_rows,
            max_import_batches,
            max_backup_size,
            max_restore_size,
            action,
            scope,
        } => {
            cmd_set(
                name,
                max_memory_mb,
                max_execution_steps,
                max_cpu_time_ms,
                max_query_complexity,
                max_join_count,
                max_sort_rows,
                max_pipeline_depth,
                max_pipeline_stages,
                max_ffi_calls,
                max_ffi_memory_mb,
                max_ffi_time_ms,
                max_training_iterations,
                max_prediction_batch_size,
                max_embedding_batch_size,
                max_vector_candidates,
                max_vector_expansions,
                max_graph_depth,
                max_graph_nodes,
                max_graph_edges,
                max_import_rows,
                max_import_batches,
                max_backup_size,
                max_restore_size,
                action,
                scope,
                fmt,
            )
            .await
        }
    }
}

async fn cmd_status(fmt: &OutputFormat) -> Result<()> {
    let engine = GovernorEngine::new(GovernorConfig::default());
    let status = engine.status().await;
    let data = OutputData::Table {
        headers: vec![
            "Field".to_string(),
            "Value".to_string(),
        ],
        rows: vec![
            vec!["Enabled".to_string(), status.enabled.to_string()],
            vec![
                "Active Executions".to_string(),
                status.active_executions.to_string(),
            ],
            vec![
                "Total Violations".to_string(),
                status.total_violations.to_string(),
            ],
            vec![
                "Blocked".to_string(),
                status.blocked_count.to_string(),
            ],
            vec![
                "Throttled".to_string(),
                status.throttled_count.to_string(),
            ],
            vec![
                "Policies Loaded".to_string(),
                status.policies_loaded.to_string(),
            ],
            vec![
                "Uptime (s)".to_string(),
                status.uptime_seconds.to_string(),
            ],
        ],
    };
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn cmd_policies(_name: Option<String>, fmt: &OutputFormat) -> Result<()> {
    let engine = GovernorEngine::new(GovernorConfig::default());
    let policies = engine.policies().await;
    let mut rows = Vec::new();
    for p in &policies {
        rows.push(vec![
            p.name.clone(),
            p.scope.as_str().to_string(),
            p.scope.name().to_string(),
            format!("{:?}", p.limits.memory.max_memory_mb.unwrap_or(0)),
            format!("{:?}", p.limits.cpu.max_execution_steps.unwrap_or(0)),
            p.action.as_str().to_string(),
        ]);
    }
    if rows.is_empty() {
        let data = OutputData::Message("No policies configured.".to_string());
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }
    let data = OutputData::Table {
        headers: vec![
            "Name".to_string(),
            "Scope Type".to_string(),
            "Scope".to_string(),
            "Max Memory (MB)".to_string(),
            "Max Steps".to_string(),
            "Action".to_string(),
        ],
        rows,
    };
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn cmd_inspect(execution_id: String, fmt: &OutputFormat) -> Result<()> {
    let id = uuid::Uuid::parse_str(&execution_id)
        .map_err(|e| crate::Error::InvalidRequest(format!("Invalid execution ID: {}", e)))?;
    let engine = GovernorEngine::new(GovernorConfig::default());
    match engine.get_execution(id).await {
        Some(ctx) => {
            let data = OutputData::Table {
                headers: vec!["Field".to_string(), "Value".to_string()],
                rows: vec![
                    vec!["Execution ID".to_string(), ctx.execution_id.to_string()],
                    vec!["Namespace".to_string(), ctx.namespace.clone()],
                    vec![
                        "Workload Type".to_string(),
                        ctx.workload_type.as_str().to_string(),
                    ],
                    vec![
                        "Action".to_string(),
                        ctx.action.as_str().to_string(),
                    ],
                    vec![
                        "Created".to_string(),
                        ctx.created_at.to_rfc3339(),
                    ],
                    vec![
                        "Elapsed (ms)".to_string(),
                        ctx.elapsed_ms().to_string(),
                    ],
                ],
            };
            println!("{}", format_output(&data, *fmt));
        }
        None => {
            let data =
                OutputData::Message(format!("Execution not found: {}", execution_id));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_metrics(watch: bool, interval: u64, fmt: &OutputFormat) -> Result<()> {
    let engine = GovernorEngine::new(GovernorConfig::default());
    loop {
        let metrics = engine.metrics_snapshot().await;
        let data = OutputData::Table {
            headers: vec!["Metric".to_string(), "Value".to_string()],
            rows: vec![
                vec![
                    "Active Executions".to_string(),
                    metrics.active_executions.to_string(),
                ],
                vec![
                    "Blocked Total".to_string(),
                    metrics.blocked_total.to_string(),
                ],
                vec![
                    "Throttled Total".to_string(),
                    metrics.throttled_total.to_string(),
                ],
                vec![
                    "Policy Violations".to_string(),
                    metrics.policy_violations_total.to_string(),
                ],
            ],
        };
        println!("{}", format_output(&data, *fmt));
        if !watch {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
    Ok(())
}

async fn cmd_violations(
    _last: Option<String>,
    _workload: Option<String>,
    _limit: Option<usize>,
    fmt: &OutputFormat,
) -> Result<()> {
    let engine = GovernorEngine::new(GovernorConfig::default());
    let violations = engine.list_violations().await;
    if violations.is_empty() {
        let data = OutputData::Message("No violations recorded.".to_string());
        println!("{}", format_output(&data, *fmt));
        return Ok(());
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for v in violations.iter().take(50) {
        rows.push(vec![
            v.id.to_string(),
            v.execution_id.to_string(),
            v.namespace.clone(),
            v.workload_type.as_str().to_string(),
            v.limit_name.clone(),
            v.limit_value.clone(),
            v.usage_value.clone(),
            v.action.as_str().to_string(),
            v.timestamp.to_rfc3339(),
        ]);
    }
    let data = OutputData::Table {
        headers: vec![
            "ID".to_string(),
            "Execution".to_string(),
            "Namespace".to_string(),
            "Workload".to_string(),
            "Limit".to_string(),
            "Limit Value".to_string(),
            "Usage".to_string(),
            "Action".to_string(),
            "Timestamp".to_string(),
        ],
        rows,
    };
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_set(
    name: String,
    max_memory_mb: Option<u64>,
    max_execution_steps: Option<u64>,
    max_cpu_time_ms: Option<u64>,
    max_query_complexity: Option<u32>,
    max_join_count: Option<u32>,
    max_sort_rows: Option<u64>,
    max_pipeline_depth: Option<u32>,
    max_pipeline_stages: Option<u32>,
    max_ffi_calls: Option<u64>,
    max_ffi_memory_mb: Option<u64>,
    max_ffi_time_ms: Option<u64>,
    max_training_iterations: Option<u64>,
    max_prediction_batch_size: Option<u64>,
    max_embedding_batch_size: Option<u64>,
    max_vector_candidates: Option<u64>,
    max_vector_expansions: Option<u64>,
    max_graph_depth: Option<u32>,
    max_graph_nodes: Option<u64>,
    max_graph_edges: Option<u64>,
    max_import_rows: Option<u64>,
    max_import_batches: Option<u64>,
    max_backup_size: Option<u64>,
    max_restore_size: Option<u64>,
    action: String,
    scope: String,
    fmt: &OutputFormat,
) -> Result<()> {
    let action = match action.to_lowercase().as_str() {
        "block" => EnforcementAction::Block,
        "throttle" => EnforcementAction::Throttle,
        "warn" => EnforcementAction::Warn,
        _ => EnforcementAction::Monitor,
    };
    let limits = ExecutionLimits {
        memory: crate::governor::MemoryLimits { max_memory_mb },
        cpu: crate::governor::CpuLimits {
            max_execution_steps,
            max_cpu_time_ms,
        },
        query_complexity: crate::governor::QueryComplexityLimits {
            max_query_complexity,
            max_join_count,
            max_sort_rows,
        },
        pipeline: crate::governor::PipelineLimits {
            max_pipeline_depth,
            max_pipeline_stages,
        },
        ffi: crate::governor::FfiLimits {
            max_ffi_calls,
            max_ffi_memory_mb,
            max_ffi_time_ms,
        },
        aiml: crate::governor::AimlLimits {
            max_training_iterations,
            max_prediction_batch_size,
            max_embedding_batch_size,
        },
        vector: crate::governor::VectorLimits {
            max_vector_candidates,
            max_vector_expansions,
        },
        graph: crate::governor::GraphLimits {
            max_graph_depth,
            max_graph_nodes,
            max_graph_edges,
        },
        migration: crate::governor::MigrationLimits {
            max_import_rows,
            max_import_batches,
        },
        backup: crate::governor::BackupLimits {
            max_backup_size,
            max_restore_size,
        },
    };
    let engine = GovernorEngine::new(GovernorConfig::default());
    engine
        .update_policy(&name, limits, action, scope)
        .await;
    let data = OutputData::Message(format!("Policy '{}' updated.", name));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
