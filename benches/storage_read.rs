use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use primusdb::storage::document::DocumentEngine;
use primusdb::storage::keyvalue::KeyValueEngine;
use primusdb::storage::StorageEngine;
use primusdb::transaction::{IsolationLevel, Transaction, TransactionStatus};
use primusdb::{CompressionType, NetworkConfig, PrimusDBConfig, SecurityConfig, StorageConfig};
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
        },
        security: SecurityConfig {
            encryption_enabled: false,
            key_rotation_interval: 86400,
            auth_required: false,
        },
        cluster: primusdb::ClusterConfig {
            enabled: false,
            node_id: "bench".to_string(),
            discovery_servers: vec![],
        },
        namespaces: Default::default(),
        federation: None,
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

// ── Key-Value Engine Benchmarks ──────────────────────────────────

fn bench_kv_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("kv_set");

    group.sample_size(100);

    group.bench_function("put_document", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = KeyValueEngine::new(&config).unwrap();
                engine.create_database("bench").unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let data = serde_json::json!({"key": "value", "num": 42});
                black_box(engine.put_document("bench", "doc1", data).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_kv_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("kv_get");

    group.sample_size(100);

    group.bench_function("get_document_existing", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = KeyValueEngine::new(&config).unwrap();
                engine.create_database("bench").unwrap();
                engine
                    .put_document("bench", "doc1", serde_json::json!({"key": "value"}))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                black_box(engine.get_document("bench", "doc1").unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Document Engine Benchmarks ───────────────────────────────────

fn bench_doc_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("doc_insert");

    group.sample_size(100);

    group.bench_function("insert_one", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(DocumentEngine::new(&config).unwrap());
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

fn bench_doc_query(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("doc_query");

    group.sample_size(100);

    group.bench_function("select_all", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = Arc::new(DocumentEngine::new(&config).unwrap());
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

criterion_group!(
    benches,
    bench_kv_set,
    bench_kv_get,
    bench_doc_insert,
    bench_doc_query,
);
criterion_main!(benches);
