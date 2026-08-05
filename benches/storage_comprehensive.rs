use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use primusdb::storage::columnar::ColumnarEngine;
use primusdb::storage::document::DocumentEngine;
use primusdb::storage::keyvalue::KeyValueEngine;
use primusdb::storage::relational::RelationalEngine;
use primusdb::storage::timeseries::TimeSeriesEngine;
use primusdb::storage::vector::VectorEngine;
use primusdb::storage::StorageEngine;
use primusdb::transaction::{IsolationLevel, Transaction, TransactionStatus};
use primusdb::{
    ClusterConfig, CompressionType, NamespaceConfig, NetworkConfig, PrimusDBConfig, SecurityConfig,
    StorageConfig,
};

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

fn make_schema() -> primusdb::storage::Schema {
    primusdb::storage::Schema {
        fields: vec![],
        indexes: vec![],
        constraints: vec![],
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

const RECORD_COUNT: usize = 1000;

// ── Columnar Engine Benchmarks ───────────────────────────────────

fn bench_columnar_insert_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_columnar_insert");
    group.sample_size(10);

    group.bench_function("insert_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = ColumnarEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "value": format!("v_{}", i), "score": i as f64 * 1.5});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_columnar_read_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_columnar_read");
    group.sample_size(10);

    group.bench_function("select_all_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = ColumnarEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "value": format!("v_{}", i)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", None, RECORD_COUNT as u64, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_columnar_query_filtered(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_columnar_query");
    group.sample_size(10);

    group.bench_function("select_filtered", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = ColumnarEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "category": format!("cat_{}", i % 5), "value": i as f64});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let conditions = serde_json::json!({"category": "cat_2"});
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", Some(&conditions), 100, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Relational Engine Benchmarks ─────────────────────────────────

fn bench_relational_insert_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_relational_insert");
    group.sample_size(10);

    group.bench_function("insert_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = RelationalEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "name": format!("user_{}", i), "age": 20 + (i % 50)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_relational_read_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_relational_read");
    group.sample_size(10);

    group.bench_function("select_all_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = RelationalEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "name": format!("user_{}", i)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", None, RECORD_COUNT as u64, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_relational_query_filtered(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_relational_query");
    group.sample_size(10);

    group.bench_function("select_filtered", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = RelationalEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "status": if i % 2 == 0 { "active" } else { "inactive" }});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let conditions = serde_json::json!({"status": "active"});
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", Some(&conditions), 100, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Document Engine Benchmarks ───────────────────────────────────

fn bench_document_insert_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_document_insert");
    group.sample_size(10);

    group.bench_function("insert_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = DocumentEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "content": format!("document_{}", i), "tags": ["tag1", "tag2"]});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_document_read_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_document_read");
    group.sample_size(10);

    group.bench_function("select_all_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = DocumentEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "content": format!("doc_{}", i)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", None, RECORD_COUNT as u64, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_document_query_filtered(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_document_query");
    group.sample_size(10);

    group.bench_function("select_filtered", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = DocumentEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data =
                        serde_json::json!({"id": i, "type": if i % 3 == 0 { "A" } else { "B" }});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let conditions = serde_json::json!({"type": "A"});
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", Some(&conditions), 100, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Vector Engine Benchmarks ─────────────────────────────────────

fn bench_vector_insert_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_vector_insert");
    group.sample_size(10);

    group.bench_function("insert_1000_128dim", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = VectorEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let vector = random_vector(128);
                    let data = serde_json::json!({"vector": vector, "id": i});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_vector_search_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_vector_search");
    group.sample_size(10);

    group.bench_function("search_1000_128dim_top10", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = VectorEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let vector = random_vector(128);
                    let data = serde_json::json!({"vector": vector, "id": i});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                let query_vec = random_vector(128);
                (engine, tmpdir, query_vec)
            },
            |(engine, _, query_vec)| {
                let conditions = serde_json::json!({"query_vector": query_vec});
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", Some(&conditions), 10, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── Key-Value Engine Benchmarks ──────────────────────────────────

fn bench_kv_insert_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("comprehensive_kv_insert");
    group.sample_size(10);

    group.bench_function("insert_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = KeyValueEngine::new(&config).unwrap();
                engine.create_database("bench").unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "value": format!("v_{}", i), "score": i as f64});
                    black_box(engine.put_document("bench", &format!("doc_{}", i), data).unwrap());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_kv_read_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("comprehensive_kv_read");
    group.sample_size(10);

    group.bench_function("read_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = KeyValueEngine::new(&config).unwrap();
                engine.create_database("bench").unwrap();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"id": i, "value": format!("v_{}", i)});
                    engine
                        .put_document("bench", &format!("doc_{}", i), data)
                        .unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                for i in 0..RECORD_COUNT {
                    black_box(engine.get_document("bench", &format!("doc_{}", i)).unwrap());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── TimeSeries Engine Benchmarks ─────────────────────────────────

fn bench_timeseries_insert_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_timeseries_insert");
    group.sample_size(10);

    group.bench_function("insert_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = TimeSeriesEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"timestamp": 1000000 + i as i64, "metric": i as f64 * 1.1, "device": format!("device_{}", i % 10)});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_timeseries_read_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_timeseries_read");
    group.sample_size(10);

    group.bench_function("select_all_1000", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = TimeSeriesEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data =
                        serde_json::json!({"timestamp": 1000000 + i as i64, "metric": i as f64});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", None, RECORD_COUNT as u64, 0, &tx))
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_timeseries_query_filtered(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("comprehensive_timeseries_query");
    group.sample_size(10);

    group.bench_function("select_filtered", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = TimeSeriesEngine::new(&config).unwrap();
                rt.block_on(engine.create_table("bench", &make_schema()))
                    .unwrap();
                let tx = make_tx();
                for i in 0..RECORD_COUNT {
                    let data = serde_json::json!({"timestamp": 1000000 + i as i64, "metric": i as f64, "host": if i % 2 == 0 { "web1" } else { "web2" }});
                    rt.block_on(engine.insert("bench", &data, &tx)).unwrap();
                }
                (engine, tmpdir)
            },
            |(engine, _)| {
                let conditions = serde_json::json!({"host": "web1"});
                let tx = make_tx();
                black_box(
                    rt.block_on(engine.select("bench", Some(&conditions), 100, 0, &tx))
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
    bench_columnar_insert_batch,
    bench_columnar_read_batch,
    bench_columnar_query_filtered,
    bench_relational_insert_batch,
    bench_relational_read_batch,
    bench_relational_query_filtered,
    bench_document_insert_batch,
    bench_document_read_batch,
    bench_document_query_filtered,
    bench_vector_insert_batch,
    bench_vector_search_batch,
    bench_kv_insert_batch,
    bench_kv_read_batch,
    bench_timeseries_insert_batch,
    bench_timeseries_read_batch,
    bench_timeseries_query_filtered,
);
criterion_main!(benches);
