use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use primusdb::cache::{CacheConfig, MemoryCache};

fn bench_cache_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_put");

    group.sample_size(100);

    group.bench_function("put_1kb", |b| {
        b.iter_batched(
            || {
                let cache = MemoryCache::new(CacheConfig::default()).unwrap();
                let data = vec![0u8; 1024];
                (cache, data)
            },
            |(mut cache, data)| {
                black_box(cache.put("key1", &data).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cache_get_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_get");

    group.sample_size(100);

    group.bench_function("get_hit_1kb", |b| {
        b.iter_batched(
            || {
                let mut cache = MemoryCache::new(CacheConfig::default()).unwrap();
                let data = vec![0xABu8; 1024];
                cache.put("key1", &data).unwrap();
                (cache, data)
            },
            |(mut cache, _)| {
                let _ = black_box(cache.get("key1").unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cache_get_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_get_miss");

    group.sample_size(100);

    group.bench_function("get_miss", |b| {
        b.iter_batched(
            || MemoryCache::new(CacheConfig::default()).unwrap(),
            |mut cache| {
                let _ = black_box(cache.get("nonexistent").unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cache_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_contains");

    group.sample_size(100);

    group.bench_function("contains_true", |b| {
        b.iter_batched(
            || {
                let mut cache = MemoryCache::new(CacheConfig::default()).unwrap();
                cache.put("key1", b"hello").unwrap();
                cache
            },
            |cache| {
                let _ = black_box(cache.contains("key1"));
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cache_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_eviction");

    group.sample_size(100);

    group.bench_function("evict_oldest", |b| {
        b.iter_batched(
            || {
                let config = CacheConfig {
                    max_memory: 2048,
                    lru_enabled: true,
                    ..CacheConfig::default()
                };
                let cache = MemoryCache::new(config).unwrap();
                cache
            },
            |mut cache| {
                // Insert entries until eviction kicks in
                for i in 0..100 {
                    let data = vec![0xFFu8; 512];
                    let key = format!("key_{}", i);
                    let _ = cache.put(&key, &data);
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_put,
    bench_cache_get_hit,
    bench_cache_get_miss,
    bench_cache_contains,
    bench_cache_eviction,
);
criterion_main!(benches);
