/*!
# Memory Cache - High-Performance Compressed Caching

The core memory cache implementation providing compressed storage, LRU eviction, and advanced search capabilities.

## Features

- **LZ4 Compression**: Automatic data compression with minimal overhead
- **LRU Eviction**: Intelligent cache management
- **Corruption Prevention**: CRC32 checksums for data integrity
- **Compressed Search**: Pattern matching without decompression
- **Bloom Filters**: Fast rejection for search queries
- **Thread Safety**: Concurrent access with fine-grained locking

## Usage

```rust
use primusdb::cache::{MemoryCache, CacheConfig, CompressionLevel};

let config = CacheConfig {
    max_memory: 1024 * 1024 * 1024, // 1GB
    compression_enabled: true,
    compression_level: CompressionLevel::Balanced,
    enable_search: true,
    corruption_check: true,
    lru_enabled: true,
    bloom_filter_enabled: true,
};

let mut cache = MemoryCache::new(config)?;

// Store data
cache.put("user:123", b"{\"name\":\"Alice\"}")?;

// Retrieve data
let data = cache.get("user:123")?;

// Search in cache
let results = cache.search("Alice", 10)?;
```

## Architecture

The cache uses a multi-layered approach:

1. **Storage Layer**: HashMap-based storage with compressed data
2. **Compression Layer**: LZ4 compression with checksums
3. **Search Layer**: Bloom filters and indexes for fast search
4. **Eviction Layer**: LRU tracking for memory management
5. **Statistics Layer**: Performance monitoring and metrics
*/

use super::compression::{CompressionEngine, CompressionError, CompressionLevel};
use super::search::{CompressedSearch, SearchError, SearchResult};
use lru::LruCache;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum memory usage in bytes
    pub max_memory: u64,
    /// Enable LZ4 compression
    pub compression_enabled: bool,
    /// Compression level (Fast/Balanced/High)
    pub compression_level: CompressionLevel,
    /// Enable compressed search capabilities
    pub enable_search: bool,
    /// Enable CRC32 corruption checking
    pub corruption_check: bool,
    /// Enable LRU eviction
    pub lru_enabled: bool,
    /// Enable bloom filters for search
    pub bloom_filter_enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory: 512 * 1024 * 1024, // 512MB
            compression_enabled: true,
            compression_level: CompressionLevel::Balanced,
            enable_search: true,
            corruption_check: true,
            lru_enabled: true,
            bloom_filter_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Compressed data
    pub data: Vec<u8>,
    /// Original uncompressed size
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Timestamp when entry was created
    pub created_at: Instant,
    /// Last access time
    pub accessed_at: Instant,
    /// Access count
    pub access_count: u64,
    /// CRC32 checksum
    pub checksum: u32,
}

#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Total entries in cache
    pub entries: usize,
    /// Current memory usage
    pub memory_used: usize,
    /// Peak memory usage
    pub memory_peak: usize,
    /// Cache hit count
    pub hits: u64,
    /// Cache miss count
    pub misses: u64,
    /// Hit rate percentage
    pub hit_rate: f64,
    /// Compression ratio percentage
    pub compression_ratio: f64,
    /// Average access time in microseconds
    pub avg_access_time_us: f64,
    /// Total evictions
    pub evictions: u64,
    /// Corruption detections
    pub corruptions_detected: u64,
}

pub struct MemoryCache {
    config: CacheConfig,
    storage: RwLock<HashMap<String, CacheEntry>>,
    lru: RwLock<LruCache<String, ()>>,
    compression: CompressionEngine,
    search: CompressedSearch,
    stats: RwLock<CacheStatistics>,
}

impl MemoryCache {
    /// Create a new memory cache with the given configuration
    pub fn new(config: CacheConfig) -> Result<Self, CacheError> {
        let compression = CompressionEngine::new(config.compression_level);

        let stats = CacheStatistics {
            entries: 0,
            memory_used: 0,
            memory_peak: 0,
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            compression_ratio: 0.0,
            avg_access_time_us: 0.0,
            evictions: 0,
            corruptions_detected: 0,
        };

        Ok(Self {
            config,
            storage: RwLock::new(HashMap::new()),
            lru: RwLock::new(LruCache::unbounded()),
            compression,
            search: CompressedSearch::new(),
            stats: RwLock::new(stats),
        })
    }

    /// Store data in the cache
    pub fn put(&mut self, key: &str, data: &[u8]) -> Result<(), CacheError> {
        let start_time = Instant::now();

        // Compress data if enabled
        let (compressed_data, original_size, compressed_size, checksum) =
            if self.config.compression_enabled {
                let compressed = self.compression.compress(data)?;
                let checksum = self.calculate_checksum(&compressed);
                (compressed.clone(), data.len(), compressed.len(), checksum)
            } else {
                let checksum = self.calculate_checksum(data);
                (data.to_vec(), data.len(), data.len(), checksum)
            };

        // Check memory limits
        self.enforce_memory_limits(compressed_data.len())?;

        // Create cache entry
        let entry = CacheEntry {
            data: compressed_data.clone(),
            original_size,
            compressed_size,
            created_at: Instant::now(),
            accessed_at: Instant::now(),
            access_count: 0,
            checksum,
        };

        // Store in cache
        {
            let mut storage = self.storage.write().unwrap();
            storage.insert(key.to_string(), entry);
        }

        // Update LRU if enabled
        if self.config.lru_enabled {
            let mut lru = self.lru.write().unwrap();
            lru.put(key.to_string(), ());
        }

        // Index for search if enabled
        if self.config.enable_search {
            self.search.index_data(key, &compressed_data)?;
        }

        // Update statistics
        self.update_stats_put(compressed_data.len(), start_time.elapsed());

        Ok(())
    }

    /// Retrieve data from the cache
    pub fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let start_time = Instant::now();

        let storage = self.storage.read().unwrap();
        if let Some(entry) = storage.get(key) {
            // Verify checksum if corruption checking is enabled
            if self.config.corruption_check {
                let actual_checksum = self.calculate_checksum(&entry.data);
                if actual_checksum != entry.checksum {
                    let mut stats = self.stats.write().unwrap();
                    stats.corruptions_detected += 1;
                    return Err(CacheError::CorruptionDetected);
                }
            }

            // Decompress if needed
            let data = if self.config.compression_enabled {
                self.compression.decompress(&entry.data)?
            } else {
                entry.data.clone()
            };

            // Update access statistics
            drop(storage);
            self.update_entry_access(key);

            // Update statistics
            self.update_stats_hit(start_time.elapsed());

            Ok(Some(data))
        } else {
            self.update_stats_miss();
            Ok(None)
        }
    }

    /// Check if key exists in cache
    pub fn contains(&self, key: &str) -> bool {
        self.storage.read().unwrap().contains_key(key)
    }

    /// Remove entry from cache
    pub fn remove(&mut self, key: &str) -> Result<bool, CacheError> {
        let removed = {
            let mut storage = self.storage.write().unwrap();
            storage.remove(key).is_some()
        };

        if removed {
            if self.config.lru_enabled {
                let mut lru = self.lru.write().unwrap();
                lru.pop(key);
            }
            self.search.remove_index(key);
            self.update_stats_remove();
        }

        Ok(removed)
    }

    /// Clear entire cache
    pub fn clear(&mut self) -> Result<(), CacheError> {
        let mut storage = self.storage.write().unwrap();
        let mut lru = self.lru.write().unwrap();

        storage.clear();
        lru.clear();

        let mut stats = self.stats.write().unwrap();
        stats.entries = 0;
        stats.memory_used = 0;

        Ok(())
    }

    /// Search for patterns in cached data
    pub fn search(&self, pattern: &str, limit: usize) -> Result<Vec<SearchResult>, CacheError> {
        if !self.config.enable_search {
            return Err(CacheError::SearchNotEnabled);
        }

        let results = self.search.search_pattern(pattern, limit)?;
        Ok(results)
    }

    /// Get cache statistics
    pub fn get_statistics(&self) -> CacheStatistics {
        let mut stats = self.stats.read().unwrap().clone();
        let total_requests = stats.hits + stats.misses;
        stats.hit_rate = if total_requests > 0 {
            (stats.hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let storage = self.storage.read().unwrap();
        let total_original: usize = storage.values().map(|e| e.original_size).sum();
        let total_compressed: usize = storage.values().map(|e| e.compressed_size).sum();

        stats.compression_ratio = if total_original > 0 {
            (total_compressed as f64 / total_original as f64) * 100.0
        } else {
            0.0
        };

        stats
    }

    /// Get cache size (number of entries)
    pub fn size(&self) -> usize {
        self.storage.read().unwrap().len()
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.stats.read().unwrap().memory_used
    }

    // Private methods

    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    fn enforce_memory_limits(&mut self, new_data_size: usize) -> Result<(), CacheError> {
        {
            let stats = self.stats.read().unwrap();
            let projected_usage = stats.memory_used + new_data_size;

            if projected_usage > self.config.max_memory as usize {
                // Need to evict entries
                drop(stats); // Release the read lock before calling evict_to_fit
                self.evict_to_fit(new_data_size)?;
            }
        }

        Ok(())
    }

    fn evict_to_fit(&mut self, required_space: usize) -> Result<(), CacheError> {
        if !self.config.lru_enabled {
            return Err(CacheError::OutOfMemory);
        }

        let mut lru = self.lru.write().unwrap();
        let mut storage = self.storage.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        while stats.memory_used + required_space > self.config.max_memory as usize
            && !lru.is_empty()
        {
            if let Some((key, _)) = lru.pop_lru() {
                if let Some(entry) = storage.remove(&key) {
                    stats.memory_used = stats.memory_used.saturating_sub(entry.compressed_size);
                    stats.entries -= 1;
                    stats.evictions += 1;
                    self.search.remove_index(&key);
                }
            }
        }

        if stats.memory_used + required_space > self.config.max_memory as usize {
            return Err(CacheError::OutOfMemory);
        }

        Ok(())
    }

    fn update_entry_access(&self, key: &str) {
        if let Ok(mut storage) = self.storage.write() {
            if let Some(entry) = storage.get_mut(key) {
                entry.accessed_at = Instant::now();
                entry.access_count += 1;
            }
        }

        if self.config.lru_enabled {
            if let Ok(mut lru) = self.lru.write() {
                lru.get(key); // Update LRU position
            }
        }
    }

    fn update_stats_put(&self, data_size: usize, duration: Duration) {
        let mut stats = self.stats.write().unwrap();
        stats.memory_used += data_size;
        if stats.memory_used > stats.memory_peak {
            stats.memory_peak = stats.memory_used;
        }
        stats.entries += 1;
        stats.avg_access_time_us = (stats.avg_access_time_us + duration.as_micros() as f64) / 2.0;
    }

    fn update_stats_hit(&self, duration: Duration) {
        let mut stats = self.stats.write().unwrap();
        stats.hits += 1;
        stats.avg_access_time_us = (stats.avg_access_time_us + duration.as_micros() as f64) / 2.0;
    }

    fn update_stats_miss(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.misses += 1;
    }

    fn update_stats_remove(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.entries = stats.entries.saturating_sub(1);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Compression error: {0}")]
    Compression(#[from] CompressionError),
    #[error("Search error: {0}")]
    Search(#[from] SearchError),
    #[error("Out of memory")]
    OutOfMemory,
    #[error("Data corruption detected")]
    CorruptionDetected,
    #[error("Search functionality not enabled")]
    SearchNotEnabled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_cache_operations() {
        let config = CacheConfig {
            max_memory: 1024 * 1024, // 1MB
            compression_enabled: false,
            compression_level: CompressionLevel::Fast,
            enable_search: false,
            corruption_check: true,
            lru_enabled: true,
            bloom_filter_enabled: false,
        };

        let mut cache = MemoryCache::new(config).unwrap();

        // Test put and get
        let data = b"Hello, World!";
        cache.put("test_key", data).unwrap();

        let retrieved = cache.get("test_key").unwrap().unwrap();
        assert_eq!(retrieved, data);

        // Test contains
        assert!(cache.contains("test_key"));
        assert!(!cache.contains("nonexistent"));

        // Test statistics
        let stats = cache.get_statistics();
        assert_eq!(stats.entries, 1);
        assert!(stats.memory_used > 0);
    }

    #[test]
    fn test_compression() {
        let config = CacheConfig {
            max_memory: 1024 * 1024,
            compression_enabled: true,
            compression_level: CompressionLevel::Fast,
            enable_search: false,
            corruption_check: true,
            lru_enabled: true,
            bloom_filter_enabled: false,
        };

        let mut cache = MemoryCache::new(config).unwrap();

        // Test with compressible data
        let data = vec![b'A'; 10000]; // Repetitive data
        cache.put("compressed_key", &data).unwrap();

        let retrieved = cache.get("compressed_key").unwrap().unwrap();
        assert_eq!(retrieved, data);

        let stats = cache.get_statistics();
        assert!(stats.compression_ratio < 100.0); // Should be compressed
    }

    #[test]
    fn test_memory_limits() {
        let config = CacheConfig {
            max_memory: 100, // Very small limit
            compression_enabled: false,
            compression_level: CompressionLevel::Fast,
            enable_search: false,
            corruption_check: false,
            lru_enabled: true,
            bloom_filter_enabled: false,
        };

        let mut cache = MemoryCache::new(config).unwrap();

        // Try to store data larger than limit
        let result = cache.put("large_key", &vec![0; 200]);
        assert!(matches!(result, Err(CacheError::OutOfMemory)));
    }

    #[test]
    fn test_remove_and_clear() {
        let config = CacheConfig::default();
        let mut cache = MemoryCache::new(config).unwrap();

        cache.put("key1", b"data1").unwrap();
        cache.put("key2", b"data2").unwrap();

        assert_eq!(cache.size(), 2);

        cache.remove("key1").unwrap();
        assert_eq!(cache.size(), 1);
        assert!(!cache.contains("key1"));

        cache.clear().unwrap();
        assert_eq!(cache.size(), 0);
    }
}
