/*!
# Compressed Search Engine - Pattern Matching over Cached Values

Indexes cached (LZ4-compressed) values and answers pattern searches against
those indexes, using bloom filters for fast rejection of non-matching keys.

> **Note**: indexing requires decompressing each cached value once (the
> payload is decoded with an LZ4 frame decoder, minus its trailing CRC32
> checksum). The resulting term index is then searched without repeated
> decompression.

## Features

- **Bloom Filters**: Fast rejection of non-matching data
- **Term Indexes**: Per-key term -> position maps for exact matching
- **Regex Search**: `search_regex` over indexed terms
- **Wildcard Support**: Basic wildcard pattern matching

## Usage

```ignore
use primusdb::cache::search::CompressedSearch;

let search = CompressedSearch::new();

// Index compressed data (decompresses once to build the term index)
search.index_data("key1", &compressed_data)?;

// Search for patterns
let results = search.search_pattern("Alice", 100)?;
```
*/

use bloom::{BloomFilter, ASMS};
use regex::Regex;
use std::collections::HashMap;
use std::sync::RwLock;

/// Search engine over indexed cache data using bloom filters and term indexes.
///
/// Decompresses cached values to build a per-key term index and a bloom
/// filter that rejects non-matching keys quickly during search.
pub struct CompressedSearch {
    indexes: RwLock<HashMap<String, SearchIndex>>,
    bloom_filters: RwLock<HashMap<String, BloomFilter>>,
}

/// Per-key term index used for pattern matching.
struct SearchIndex {
    /// Positions of every searchable term occurrence
    positions: Vec<usize>,
    /// Mapping of term to its positions within the entry
    terms: HashMap<String, Vec<usize>>,
    /// Bloom filter for fast rejection of non-matching keys
    bloom_filter: Option<BloomFilter>,
}

impl Default for CompressedSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressedSearch {
    /// Create a new compressed search engine
    pub fn new() -> Self {
        Self {
            indexes: RwLock::new(HashMap::new()),
            bloom_filters: RwLock::new(HashMap::new()),
        }
    }

    /// Index compressed data for fast searching
    pub fn index_data(&self, key: &str, compressed_data: &[u8]) -> Result<(), SearchError> {
        // Extract searchable terms from compressed data
        // This is a simplified implementation - in practice, you'd need
        // LZ4-aware text extraction
        let searchable_text = self.extract_searchable_text(compressed_data)?;

        // Build term index
        let mut terms = HashMap::new();
        let mut bloom = BloomFilter::with_rate(0.01, 10000); // 1% false positive rate

        for (pos, word) in searchable_text.split_whitespace().enumerate() {
            terms
                .entry(word.to_string())
                .or_insert_with(Vec::new)
                .push(pos);
            bloom.insert(&word.to_string());
        }

        let index = SearchIndex {
            positions: (0..searchable_text.len()).collect(),
            terms,
            bloom_filter: Some(bloom),
        };

        self.indexes.write().unwrap().insert(key.to_string(), index);
        Ok(())
    }

    /// Search for a pattern in compressed data
    pub fn search_pattern(
        &self,
        pattern: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let mut results = Vec::new();

        // First pass: bloom filter check
        let indexes = self.indexes.read().unwrap();
        let _bloom_filters = self.bloom_filters.read().unwrap();

        for (key, index) in indexes.iter() {
            // Check bloom filter first
            if let Some(bloom) = &index.bloom_filter {
                if !bloom.contains(&pattern.to_string()) {
                    continue; // Fast rejection
                }
            }

            // Check term index
            if let Some(positions) = index.terms.get(pattern) {
                for &pos in positions.iter().take(limit - results.len()) {
                    results.push(SearchResult {
                        key: key.clone(),
                        position: pos,
                        matched_text: pattern.to_string(),
                        score: 1.0, // Simplified scoring
                    });

                    if results.len() >= limit {
                        break;
                    }
                }
            }

            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }

    /// Search with regex patterns
    pub fn search_regex(
        &self,
        regex_pattern: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let regex = Regex::new(regex_pattern)?;

        let mut results = Vec::new();
        let indexes = self.indexes.read().unwrap();

        for (key, index) in indexes.iter() {
            if results.len() >= limit {
                break;
            }

            // Search through terms for regex match
            for (pos, term) in index.terms.keys().enumerate() {
                if regex.is_match(term) {
                    results.push(SearchResult {
                        key: key.clone(),
                        position: pos,
                        matched_text: term.clone(),
                        score: 1.0,
                    });
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    /// Remove index for a key
    pub fn remove_index(&self, key: &str) {
        self.indexes.write().unwrap().remove(key);
        self.bloom_filters.write().unwrap().remove(key);
    }

    /// Get search statistics
    pub fn get_statistics(&self) -> SearchStatistics {
        let indexes = self.indexes.read().unwrap();
        let bloom_filters = self.bloom_filters.read().unwrap();

        SearchStatistics {
            indexed_keys: indexes.len(),
            bloom_filters: bloom_filters.len(),
            total_terms: indexes.values().map(|idx| idx.terms.len()).sum(),
            memory_usage: self.estimate_memory_usage(),
        }
    }

    /// Extract searchable text from compressed data
    fn extract_searchable_text(&self, compressed_data: &[u8]) -> Result<String, SearchError> {
        if compressed_data.is_empty() {
            return Ok(String::new());
        }

        // Strip trailing CRC32 checksum (last 4 bytes) if present
        let payload = if compressed_data.len() > 4 {
            &compressed_data[..compressed_data.len() - 4]
        } else {
            compressed_data
        };

        // Decompress using LZ4 frame decoder
        use std::io::Read;
        let mut decoder = lz4::Decoder::new(payload)
            .map_err(|e| SearchError::DecompressionError(e.to_string()))?;
        let mut output = String::new();
        decoder
            .read_to_string(&mut output)
            .map_err(|e| SearchError::DecompressionError(e.to_string()))?;

        Ok(output)
    }

    /// Estimate memory usage of indexes
    fn estimate_memory_usage(&self) -> usize {
        let indexes = self.indexes.read().unwrap();
        let mut total = 0;

        for index in indexes.values() {
            total += std::mem::size_of::<SearchIndex>();
            total += index.positions.len() * std::mem::size_of::<usize>();
            total += index.terms.len() * std::mem::size_of::<String>();

            for (term, positions) in &index.terms {
                total += term.len();
                total += positions.len() * std::mem::size_of::<usize>();
            }
        }

        total
    }
}

/// A single match produced by [`CompressedSearch`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    /// Key of the cached entry that matched
    pub key: String,
    /// Offset of the match within the entry
    pub position: usize,
    /// The text that matched the pattern
    pub matched_text: String,
    /// Relevance score of the match (0.0 to 1.0)
    pub score: f64,
}

/// Aggregate statistics about the search index.
#[derive(Debug, Clone)]
pub struct SearchStatistics {
    /// Number of keys with a search index
    pub indexed_keys: usize,
    /// Number of bloom filters in use
    pub bloom_filters: usize,
    /// Total number of indexed terms
    pub total_terms: usize,
    /// Estimated memory footprint of the indexes in bytes
    pub memory_usage: usize,
}

/// Errors that can occur during search operations.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// Invalid regular expression
    #[error("Regex compilation error: {0}")]
    Regex(#[from] regex::Error),
    /// An underlying I/O operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Malformed compressed payload
    #[error("Invalid data format")]
    InvalidData,
    /// Failed to decompress indexed data
    #[error("Decompression error: {0}")]
    DecompressionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_search() {
        let search = CompressedSearch::new();

        // Create mock index
        let mut indexes = search.indexes.write().unwrap();
        let mut terms = HashMap::new();
        terms.insert("test".to_string(), vec![0, 10]);
        terms.insert("data".to_string(), vec![5]);

        let index = SearchIndex {
            positions: vec![0, 1, 2, 3, 4, 5],
            terms,
            bloom_filter: None,
        };

        indexes.insert("key1".to_string(), index);
        drop(indexes);

        let results = search.search_pattern("test", 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, "key1");
        assert_eq!(results[0].matched_text, "test");
    }

    #[test]
    fn test_regex_search() {
        let search = CompressedSearch::new();
        let results = search.search_regex("test.*", 10).unwrap();
        // Should not panic, even if no results
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_statistics() {
        let search = CompressedSearch::new();
        let stats = search.get_statistics();
        assert_eq!(stats.indexed_keys, 0);
        assert_eq!(stats.memory_usage, 0);
    }
}
