use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use primusdb::storage::vector::VectorEngine;
use primusdb::storage::StorageEngine;
use primusdb::transaction::{IsolationLevel, Transaction, TransactionStatus};
use primusdb::{
    ClusterConfig, CompressionType, NetworkConfig, PrimusDBConfig, SecurityConfig, StorageConfig,
};
use std::sync::Arc;

fn make_config(tmpdir: &tempfile::TempDir) -> PrimusDBConfig {
    PrimusDBConfig {
        storage: StorageConfig {
            data_dir: tmpdir.path().join("data").to_string_lossy().to_string(),
            max_file_size: 1073741824,
            compression: CompressionType::None,
            cache_size: 104857600,
        },
        network: NetworkConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0,
            max_connections: 100,
            tls_enabled: false,
            tls_cert_path: String::new(),
            tls_key_path: String::new(),
            tls_ca_path: String::new(),
            mtls_enabled: false,
        },
        security: SecurityConfig {
            encryption_enabled: false,
            key_rotation_interval: 86400,
            auth_required: false,
            mfa_enabled: false,
        },
        cluster: ClusterConfig {
            enabled: false,
            node_id: "bench".to_string(),
            discovery_servers: vec![],
        },
        namespaces: Default::default(),
        federation: None,
        integrity: primusdb::integrity::IntegrityConfig::default(),
        hyperledger: None,
        graphql: primusdb::graphql::GraphQLConfig::default(),
        search: primusdb::search::SearchConfig::default(),
    }
}

fn make_tx() -> Transaction {
    Transaction {
        id: "bench_tx".to_string(),
        operations: vec![],
        status: TransactionStatus::Prepared,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        isolation_level: IsolationLevel::ReadCommitted,
        timeout_ms: 0,
    }
}

fn random_vector(dim: usize) -> Vec<f32> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let mut state = seed;
    (0..dim)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f32 / u32::MAX as f32
        })
        .collect()
}

// ── Vector Insert Benchmark ──────────────────────────────────────

fn bench_vector_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("vector_insert");

    group.sample_size(100);

    group.bench_function("insert_128dim", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(VectorEngine::new(&config).unwrap());
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("vec_bench", &schema))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let vector = random_vector(128);
                let data = serde_json::json!({"vector": vector, "label": "test"});
                let tx = make_tx();
                rt.block_on(engine.insert("vec_bench", &data, &tx)).unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Vector Search Benchmark ──────────────────────────────────────

fn bench_vector_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("vector_search");

    group.sample_size(100);

    group.bench_function("cosine_128dim_1000vec", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(VectorEngine::new(&config).unwrap());
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("vec_bench", &schema))
                    .unwrap();
                let tx = make_tx();
                for _ in 0..1000 {
                    let vector = random_vector(128);
                    let data = serde_json::json!({"vector": vector});
                    rt.block_on(engine.insert("vec_bench", &data, &tx)).unwrap();
                }
                let query_vec = random_vector(128);
                (engine, tmpdir, query_vec)
            },
            |(engine, _, query_vec)| {
                let conditions = serde_json::json!({"query_vector": query_vec});
                let tx = make_tx();
                let _ = black_box(
                    rt.block_on(engine.select("vec_bench", Some(&conditions), 10, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_vector_insert, bench_vector_search,);
criterion_main!(benches);
