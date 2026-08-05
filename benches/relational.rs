use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use primusdb::storage::relational::RelationalEngine;
use primusdb::storage::StorageEngine;
use primusdb::transaction::{IsolationLevel, Transaction, TransactionStatus};
use primusdb::{
    ClusterConfig, CompressionType, NamespaceConfig, NetworkConfig, PrimusDBConfig, SecurityConfig,
    StorageConfig,
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
        namespaces: NamespaceConfig::default(),
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

fn bench_relational_create_table(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("relational_create_table");

    group.sample_size(100);

    group.bench_function("create_empty", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(RelationalEngine::new(&config).unwrap());
                (engine, tmpdir)
            },
            |(engine, _)| {
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("bench", &schema)).unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_relational_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("relational_insert");

    group.sample_size(100);

    group.bench_function("insert_one", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(RelationalEngine::new(&config).unwrap());
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("bench", &schema)).unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let data = serde_json::json!({"name": "alice", "age": 30});
                let tx = make_tx();
                rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_relational_select(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("relational_select");

    group.sample_size(100);

    group.bench_function("select_100_rows", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(RelationalEngine::new(&config).unwrap());
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("bench", &schema)).unwrap();
                let tx = make_tx();
                for i in 0..100 {
                    let data = serde_json::json!({"id": i, "name": format!("user_{}", i)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                let _ = black_box(
                    rt.block_on(engine.select("bench", None, 100, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_relational_update(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("relational_update");

    group.sample_size(100);

    group.bench_function("update_all", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(RelationalEngine::new(&config).unwrap());
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("bench", &schema)).unwrap();
                let tx = make_tx();
                for i in 0..50 {
                    let data = serde_json::json!({"id": i, "name": format!("user_{}", i)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let data = serde_json::json!({"name": "updated"});
                let tx = make_tx();
                let _ = black_box(
                    rt.block_on(engine.update("bench", None, &data, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_relational_delete(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("relational_delete");

    group.sample_size(100);

    group.bench_function("delete_all", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(RelationalEngine::new(&config).unwrap());
                let schema = primusdb::storage::Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                };
                rt.block_on(engine.create_table("bench", &schema)).unwrap();
                let tx = make_tx();
                for i in 0..50 {
                    let data = serde_json::json!({"id": i, "name": format!("user_{}", i)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                let _ = black_box(rt.block_on(engine.delete("bench", None, &tx)).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_relational_create_table,
    bench_relational_insert,
    bench_relational_select,
    bench_relational_update,
    bench_relational_delete,
);
criterion_main!(benches);
