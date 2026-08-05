/*!
# Memory Cache System - In-Memory Caching

The memory cache system stores values LZ4-compressed, enforces a memory budget
with LRU eviction, verifies data integrity with CRC32 checksums, and indexes
cached values for pattern search. The public surface is [`MemoryCache`], backed
by the [`cache`], [`compression`] and [`search`] submodules.

## Architecture Overview

```text
Memory Cache Architecture
=======================================================

+-----------------------------------------------------------+
|                    Cache Manager                           |
|  +-----------------------------------------------------+  |
|  | Memory Pool:                                        |  |
|  |  * Compressed data blocks                           |  |
|  |  * Metadata (keys, sizes, checksums)                |  |
|  |  * LRU tracking                                     |  |
|  +-----------------------------------------------------+  |
|                                                           |
|  +-----------------------------------------------------+  |
|  | Compression Engine:                                 |  |
|  |  * LZ4 compression/decompression                    |  |
|  |  * Configurable compression levels                   |  |
|  |  * CRC32 corruption detection                       |  |
|  +-----------------------------------------------------+  |
|                                                           |
|  +-----------------------------------------------------+  |
|  | Search Engine:                                      |  |
|  |  * Term indexes built from cached values            |  |
|  |  * Bloom filters for fast rejection                 |  |
|  +-----------------------------------------------------+  |
+-----------------------------------------------------------+
```

## Features

- **Compressed Storage**: Values are stored LZ4-compressed; the memory savings
  depend entirely on the data (repetitive data compresses well, small or
  already-dense payloads gain little)
- **Compressed Search**: Pattern matching over an index built from cached values
- **LRU Eviction**: Least-recently-used entries evicted when the memory budget
  is exceeded (can be disabled via `lru_enabled`)
- **Corruption Prevention**: CRC32 checksums verified on read
- **Thread Safety**: Concurrent access guarded by `RwLock`s

## Usage

### Basic Cache Operations

```ignore
use primusdb::cache::{CacheConfig, MemoryCache};

// Create cache with 1GB limit and LZ4 compression
let config = CacheConfig {
    max_memory: 1024 * 1024 * 1024, // 1GB
    compression_enabled: true,
    compression_level: CompressionLevel::Fast,
    enable_search: true,
    corruption_check: true,
};

let mut cache = MemoryCache::new(config)?;

// Store compressed data
cache.put("user:123", b"{\"name\":\"Alice\",\"email\":\"alice@example.com\"}")?;

// Retrieve with automatic decompression
let data = cache.get("user:123")?;

// Search in compressed data
let results = cache.search("Alice", 100)?;
```

### Advanced Configuration

```ignore
let config = CacheConfig {
    max_memory: 512 * 1024 * 1024, // 512MB
    compression_enabled: true,
    compression_level: CompressionLevel::High,
    enable_search: true,
    corruption_check: true,
    lru_enabled: true,
    bloom_filter_enabled: true,
};
```

## Performance Characteristics

No micro-benchmarks are maintained in-tree. In practice, LZ4 is a very fast
codec: compressed payloads are small enough to keep the cache in RAM, and each
`get` pays one CRC32 verify plus one LZ4 decompress. Compression and
decompression are CPU-bound and scale with payload size, so actual latency
depends on the data being cached. Note that values are cloned (never zero-copy)
as they move between the storage map and the caller.

## Configuration Options

### CacheConfig

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| max_memory | u64 | Maximum memory usage in bytes | 512MB |
| compression_enabled | bool | Enable LZ4 compression | true |
| compression_level | CompressionLevel | Compression speed/quality trade-off | Balanced |
| enable_search | bool | Enable compressed search | true |
| corruption_check | bool | Enable CRC32 checksums | true |
| lru_enabled | bool | Enable LRU eviction | true |
| bloom_filter_enabled | bool | Enable bloom filters | true |

### CompressionLevel

- **Fast**: Highest speed, lowest ratio
- **Balanced**: Default speed/ratio compromise
- **High**: Highest ratio, slowest speed

Ratios are data-dependent; the levels only tune the LZ4 encoder speed/size
trade-off.

## Monitoring and Statistics

```ignore
let stats = cache.get_statistics();
println!("Cache hit rate: {:.2}%", stats.hit_rate);
println!("Memory usage: {} MB", stats.memory_used / 1024 / 1024);
println!("Compression ratio: {:.2}%", stats.compression_ratio);
```

## Error Handling

The cache system provides detailed error information via [`CacheError`]:

- **OutOfMemory**: Memory limit exceeded after LRU eviction
- **Compression**: LZ4 compression/decompression failure
- **CorruptionDetected**: Stored CRC32 checksum did not match on read
- **SearchNotEnabled**: Search requested but disabled in configuration

## Thread Safety

All operations are thread-safe with fine-grained locking:

- **Read Operations**: Shared locks for concurrent access
- **Write Operations**: Exclusive locks during modification
- **Search Operations**: Read locks over the index structures

## Memory Management

- **Compressed Storage**: Values are stored LZ4-compressed to minimise memory use
- **LRU Eviction**: Least recently used entries evicted when the memory budget
  is exceeded; if eviction is disabled or cannot free enough space, `put`
  returns [`CacheError::OutOfMemory`]
- The compression engine keeps a small buffer-pool field, but buffers are
  currently never returned to it, so allocation reuse is not yet effective
*/

#[allow(clippy::module_inception)]
pub mod cache;
pub mod cluster;
pub mod compression;
pub mod consensus;
pub mod hashing;
pub mod manager;
pub mod search;

pub use cache::{CacheConfig, CacheEntry, CacheStatistics, MemoryCache};
pub use compression::CompressionLevel;
pub use consensus::{CacheConsensusEngine, ConsensusConfig, ConsensusError};
pub use hashing::{HashRing, HashRingConfig};
pub use manager::{CacheCluster, ClusterConfig, ClusterError, ClusterHealth};
