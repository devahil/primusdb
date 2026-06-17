use prometheus::{Counter, Encoder, Gauge, Histogram, HistogramOpts, Registry, TextEncoder};
use std::sync::OnceLock;

// ── Metric keys used by federation / domain code ──────────

const FED_CLUSTERS_ONLINE: &str = "primusdb_federation_clusters_online";
const FED_CLUSTERS_TOTAL: &str = "primusdb_federation_clusters_total";
const FED_DOMAINS_TOTAL: &str = "primusdb_federation_domains_total";
const FED_ANNOUNCE_CYCLES: &str = "primusdb_federation_announce_cycles_total";
const FED_REPLICATIONS_TOTAL: &str = "primusdb_federation_replications_total";
const FED_REPLICATION_FAILURES: &str = "primusdb_federation_replication_failures_total";
const FED_REPLICATION_LATENCY: &str = "primusdb_federation_replication_latency_seconds";
const FED_HEALTHY_RATIO: &str = "primusdb_federation_healthy_ratio";
const DOMAIN_HEALTHY: &str = "primusdb_domain_healthy";

const KV_OPERATIONS_TOTAL: &str = "primusdb_kv_operations_total";
const KV_OPERATION_ERRORS: &str = "primusdb_kv_operation_errors_total";
const KV_OPERATION_LATENCY: &str = "primusdb_kv_operation_latency_seconds";
const KV_DATABASES_TOTAL: &str = "primusdb_kv_databases_total";
const KV_DOCUMENTS_TOTAL: &str = "primusdb_kv_documents_total";

const GOV_ACTIVE_EXECUTIONS: &str = "primusdb_governor_active_executions";
const GOV_BLOCKED_TOTAL: &str = "primusdb_governor_blocked_total";
const GOV_THROTTLED_TOTAL: &str = "primusdb_governor_throttled_total";
const GOV_VIOLATIONS_TOTAL: &str = "primusdb_governor_policy_violations_total";
const GOV_MEMORY_USAGE: &str = "primusdb_governor_memory_usage_bytes";
const GOV_CPU_TIME: &str = "primusdb_governor_cpu_time_ms";
const GOV_FFI_CALLS: &str = "primusdb_governor_ffi_calls_total";

/// All Prometheus metrics used by PrimusDB.
pub struct PrimusMetrics {
    pub registry: Registry,

    // ── Federation ────────────────────────────────────
    pub federation_clusters_online: Gauge,
    pub federation_clusters_total: Gauge,
    pub federation_domains_total: Gauge,
    pub federation_healthy_ratio: Gauge,
    pub federation_announce_cycles: Counter,

    // ── Cross-cluster replication ─────────────────────
    pub replications_total: Counter,
    pub replication_failures: Counter,
    pub replication_latency: Histogram,

    // ── Domain health ─────────────────────────────────
    pub domain_healthy: Gauge,

    // ── Key-Value engine ─────────────────────────────
    pub kv_operations_total: Counter,
    pub kv_operation_errors: Counter,
    pub kv_operation_latency: Histogram,
    pub kv_databases_total: Gauge,
    pub kv_documents_total: Gauge,

    // ── Resource Governor ────────────────────────────
    pub governor_active_executions: Gauge,
    pub governor_blocked_total: Counter,
    pub governor_throttled_total: Counter,
    pub governor_policy_violations_total: Counter,
    pub governor_memory_usage_bytes: Gauge,
    pub governor_cpu_time_ms: Gauge,
    pub governor_ffi_calls_total: Counter,
}

impl PrimusMetrics {
    fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let federation_clusters_online = Gauge::new(
            FED_CLUSTERS_ONLINE,
            "Number of federation clusters currently online",
        )?;
        registry.register(Box::new(federation_clusters_online.clone()))?;

        let federation_clusters_total = Gauge::new(
            FED_CLUSTERS_TOTAL,
            "Total number of registered federation clusters",
        )?;
        registry.register(Box::new(federation_clusters_total.clone()))?;

        let federation_domains_total =
            Gauge::new(FED_DOMAINS_TOTAL, "Total number of data domains")?;
        registry.register(Box::new(federation_domains_total.clone()))?;

        let federation_healthy_ratio = Gauge::new(
            FED_HEALTHY_RATIO,
            "Ratio of online clusters to total clusters",
        )?;
        registry.register(Box::new(federation_healthy_ratio.clone()))?;

        let federation_announce_cycles = Counter::new(
            FED_ANNOUNCE_CYCLES,
            "Total federation announce/heartbeat cycles completed",
        )?;
        registry.register(Box::new(federation_announce_cycles.clone()))?;

        let replications_total = Counter::new(
            FED_REPLICATIONS_TOTAL,
            "Total cross-cluster replication requests sent",
        )?;
        registry.register(Box::new(replications_total.clone()))?;

        let replication_failures = Counter::new(
            FED_REPLICATION_FAILURES,
            "Total cross-cluster replication failures",
        )?;
        registry.register(Box::new(replication_failures.clone()))?;

        let replication_latency = Histogram::with_opts(
            HistogramOpts::new(
                FED_REPLICATION_LATENCY,
                "Latency of cross-cluster replication in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        )?;
        registry.register(Box::new(replication_latency.clone()))?;

        let domain_healthy = Gauge::new(
            DOMAIN_HEALTHY,
            "1 if replication domain is healthy, 0 otherwise",
        )?;
        registry.register(Box::new(domain_healthy.clone()))?;

        let kv_operations_total =
            Counter::new(KV_OPERATIONS_TOTAL, "Total KV operations performed")?;
        registry.register(Box::new(kv_operations_total.clone()))?;

        let kv_operation_errors = Counter::new(KV_OPERATION_ERRORS, "Total KV operation errors")?;
        registry.register(Box::new(kv_operation_errors.clone()))?;

        let kv_operation_latency = Histogram::with_opts(
            HistogramOpts::new(KV_OPERATION_LATENCY, "Latency of KV operations in seconds")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        )?;
        registry.register(Box::new(kv_operation_latency.clone()))?;

        let kv_databases_total = Gauge::new(KV_DATABASES_TOTAL, "Total number of KV databases")?;
        registry.register(Box::new(kv_databases_total.clone()))?;

        let kv_documents_total = Gauge::new(
            KV_DOCUMENTS_TOTAL,
            "Total number of KV documents across all databases",
        )?;
        registry.register(Box::new(kv_documents_total.clone()))?;

        let governor_active_executions = Gauge::new(
            GOV_ACTIVE_EXECUTIONS,
            "Current number of active executions tracked by the Resource Governor",
        )?;
        registry.register(Box::new(governor_active_executions.clone()))?;

        let governor_blocked_total = Counter::new(
            GOV_BLOCKED_TOTAL,
            "Total number of executions blocked by the Resource Governor",
        )?;
        registry.register(Box::new(governor_blocked_total.clone()))?;

        let governor_throttled_total = Counter::new(
            GOV_THROTTLED_TOTAL,
            "Total number of executions throttled by the Resource Governor",
        )?;
        registry.register(Box::new(governor_throttled_total.clone()))?;

        let governor_policy_violations_total = Counter::new(
            GOV_VIOLATIONS_TOTAL,
            "Total number of policy violations detected by the Resource Governor",
        )?;
        registry.register(Box::new(governor_policy_violations_total.clone()))?;

        let governor_memory_usage_bytes = Gauge::new(
            GOV_MEMORY_USAGE,
            "Current memory usage in bytes tracked by the Resource Governor",
        )?;
        registry.register(Box::new(governor_memory_usage_bytes.clone()))?;

        let governor_cpu_time_ms = Gauge::new(
            GOV_CPU_TIME,
            "Total CPU time in milliseconds tracked by the Resource Governor",
        )?;
        registry.register(Box::new(governor_cpu_time_ms.clone()))?;

        let governor_ffi_calls_total = Counter::new(
            GOV_FFI_CALLS,
            "Total number of FFI calls tracked by the Resource Governor",
        )?;
        registry.register(Box::new(governor_ffi_calls_total.clone()))?;

        Ok(Self {
            registry,
            federation_clusters_online,
            federation_clusters_total,
            federation_domains_total,
            federation_healthy_ratio,
            federation_announce_cycles,
            replications_total,
            replication_failures,
            replication_latency,
            domain_healthy,
            kv_operations_total,
            kv_operation_errors,
            kv_operation_latency,
            kv_databases_total,
            kv_documents_total,
            governor_active_executions,
            governor_blocked_total,
            governor_throttled_total,
            governor_policy_violations_total,
            governor_memory_usage_bytes,
            governor_cpu_time_ms,
            governor_ffi_calls_total,
        })
    }

    /// Render all metrics in Prometheus text exposition format.
    pub fn encode(&self) -> String {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&self.registry.gather(), &mut buffer).ok();
        String::from_utf8(buffer).unwrap_or_default()
    }
}

/// Global singleton accessor.
pub fn get_metrics() -> &'static PrimusMetrics {
    static METRICS: OnceLock<PrimusMetrics> = OnceLock::new();
    METRICS.get_or_init(|| {
        PrimusMetrics::new().expect("Failed to initialize PrimusDB Prometheus metrics")
    })
}

/// Update Prometheus counters from federation / domain state.
pub fn update_federation_metrics(
    clusters_online: usize,
    clusters_total: usize,
    domains_count: usize,
) {
    let m = get_metrics();
    m.federation_clusters_online.set(clusters_online as f64);
    m.federation_clusters_total.set(clusters_total as f64);
    m.federation_domains_total.set(domains_count as f64);
    if clusters_total > 0 {
        m.federation_healthy_ratio
            .set(clusters_online as f64 / clusters_total as f64);
    }
    m.federation_announce_cycles.inc();
}

/// Record a successful replication attempt (call after ack).
pub fn record_replication(duration_secs: f64) {
    let m = get_metrics();
    m.replications_total.inc();
    m.replication_latency.observe(duration_secs);
}

/// Record a failed replication attempt.
pub fn record_replication_failure() {
    get_metrics().replication_failures.inc();
}

/// Record a KV operation (call after each KV API handler invocation).
pub fn record_kv_operation(_kind: &str, duration_secs: f64, is_error: bool) {
    let m = get_metrics();
    m.kv_operations_total.inc();
    m.kv_operation_latency.observe(duration_secs);
    if is_error {
        m.kv_operation_errors.inc();
    }
}

/// Update Resource Governor active executions gauge.
pub fn set_governor_active_executions(count: usize) {
    get_metrics()
        .governor_active_executions
        .set(count as f64);
}

/// Record a Resource Governor block event.
pub fn record_governor_block() {
    get_metrics().governor_blocked_total.inc();
}

/// Record a Resource Governor throttle event.
pub fn record_governor_throttle() {
    get_metrics().governor_throttled_total.inc();
}

/// Record a Resource Governor policy violation.
pub fn record_governor_violation() {
    get_metrics().governor_policy_violations_total.inc();
}

/// Set Resource Governor memory usage in bytes.
pub fn set_governor_memory_usage_bytes(bytes: u64) {
    get_metrics()
        .governor_memory_usage_bytes
        .set(bytes as f64);
}

/// Set Resource Governor CPU time in milliseconds.
pub fn set_governor_cpu_time_ms(ms: u64) {
    get_metrics().governor_cpu_time_ms.set(ms as f64);
}

/// Record a Resource Governor FFI call.
pub fn record_governor_ffi_call() {
    get_metrics().governor_ffi_calls_total.inc();
}

/// Update KV database/document gauges.
pub fn update_kv_db_gauges(db_count: usize, doc_count: usize) {
    let m = get_metrics();
    m.kv_databases_total.set(db_count as f64);
    m.kv_documents_total.set(doc_count as f64);
}

/// Describe all registered metrics with their names and help text.
/// Gathers from both the global default registry and the custom PrimusDB registry.
pub fn describe_metrics() -> Vec<String> {
    use std::collections::BTreeSet;

    let mut descriptions: BTreeSet<String> = BTreeSet::new();

    for mf in prometheus::gather() {
        let name = mf.get_name().to_string();
        let help = mf.get_help();
        if help.is_empty() {
            descriptions.insert(name);
        } else {
            descriptions.insert(format!("{} # {}", name, help));
        }
    }

    for mf in get_metrics().registry.gather() {
        let name = mf.get_name().to_string();
        let help = mf.get_help();
        if help.is_empty() {
            descriptions.insert(name);
        } else {
            descriptions.insert(format!("{} # {}", name, help));
        }
    }

    descriptions.into_iter().collect()
}
