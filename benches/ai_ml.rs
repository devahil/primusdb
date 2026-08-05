use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use primusdb::ai::{AIEngine, ModelType, TrainingRequest};
use primusdb::{
    ClusterConfig, CompressionType, NetworkConfig, PrimusDBConfig, SecurityConfig, StorageConfig,
};
use std::collections::HashMap;

fn make_config(_tmpdir: &tempfile::TempDir) -> PrimusDBConfig {
    PrimusDBConfig {
        storage: StorageConfig {
            data_dir: _tmpdir.path().join("data").to_string_lossy().to_string(),
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

// ── AI Train Model Benchmark ─────────────────────────────────────

fn bench_train_model(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("ai_train");

    group.sample_size(100);

    group.bench_function("linear_regression", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = AIEngine::new(&config).unwrap();
                (engine, tmpdir)
            },
            |(mut engine, _)| {
                let request = TrainingRequest {
                    table: "sales_data".to_string(),
                    model_type: ModelType::LinearRegression,
                    target_column: "revenue".to_string(),
                    feature_columns: vec!["marketing_spend".to_string(), "season".to_string()],
                    hyperparameters: {
                        let mut h = HashMap::new();
                        h.insert("learning_rate".to_string(), 0.01);
                        h
                    },
                    validation_split: 0.2,
                };
                let model = rt.block_on(engine.train_model(&request)).unwrap();
                black_box(model);
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── AI Predict Benchmark ─────────────────────────────────────────

fn bench_predict(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("ai_predict");

    group.sample_size(100);

    group.bench_function("predict_value", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = AIEngine::new(&config).unwrap();
                (engine, tmpdir)
            },
            |(engine, _)| {
                let conditions = serde_json::json!({"marketing_spend": 50000, "season": "Q1"});
                let result = rt
                    .block_on(engine.predict("sales_data", Some(&conditions)))
                    .unwrap();
                black_box(result);
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── AI Anomaly Detection Benchmark ───────────────────────────────

fn bench_anomaly_detection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("ai_anomaly");

    group.sample_size(100);

    group.bench_function("detect_anomalies", |b| {
        b.iter_batched(
            || {
                let tmpdir = tempfile::tempdir().unwrap();
                let config = make_config(&tmpdir);
                let engine = AIEngine::new(&config).unwrap();
                let records: Vec<serde_json::Value> = (0..100)
                    .map(|i| serde_json::json!({"value": i as f64, "feature1": i as f64 * 1.5}))
                    .collect();
                (engine, tmpdir, records)
            },
            |(engine, _, records)| {
                let result = rt
                    .block_on(engine.detect_anomalies("transactions", &records))
                    .unwrap();
                black_box(result);
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_train_model,
    bench_predict,
    bench_anomaly_detection,
);
criterion_main!(benches);
