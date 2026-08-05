/*
 * PrimusDB Full-Text Search Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.0.0
 */

//! Simple inverted-index full-text search engine.
//!
//! Tokenizes text fields, builds an inverted index, and supports
//! boolean queries (AND, OR, Phrase) with TF-IDF scoring.
//!
//! # Example
//!
//! ```ignore
//! use primusdb::fulltext::{FullTextIndex, SearchMode};
//!
//! let mut idx = FullTextIndex::new();
//! idx.index_document(1, "The quick brown fox");
//! idx.index_document(2, "The lazy dog");
//! let results = idx.search("quick fox", SearchMode::And);
//! ```
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Search mode for full-text queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchMode {
    /// All terms must match (intersection)
    And,
    /// Any term must match (union)
    Or,
    /// Exact phrase match
    Phrase,
}

/// A full-text search index for a single collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextIndex {
    /// Inverted index: term -> set of document IDs
    inverted_index: HashMap<String, HashSet<u64>>,
    /// Document term frequencies: doc_id -> term -> count
    doc_frequencies: HashMap<u64, HashMap<String, usize>>,
    /// Total number of documents indexed
    total_docs: u64,
    /// Optional set of stop words to filter during tokenization
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stop_words: Option<HashSet<String>>,
}

impl Default for FullTextIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl FullTextIndex {
    /// Create a new empty `FullTextIndex` with no stop words.
    pub fn new() -> Self {
        Self {
            inverted_index: HashMap::new(),
            doc_frequencies: HashMap::new(),
            total_docs: 0,
            stop_words: None,
        }
    }

    /// Create a new empty `FullTextIndex` with the given set of stop words.
    ///
    /// Stop words (e.g. "the", "a", "is") are filtered out during tokenization
    /// and will not be indexed or searched.
    pub fn with_stop_words(stop_words: HashSet<String>) -> Self {
        Self {
            inverted_index: HashMap::new(),
            doc_frequencies: HashMap::new(),
            total_docs: 0,
            stop_words: Some(stop_words),
        }
    }

    /// Index a text value for a document.
    ///
    /// The text is tokenized into lowercase terms. Each term's document frequency
    /// is recorded in the inverted index. If the document already exists in the
    /// index, its previous terms are replaced.
    pub fn index_document(&mut self, doc_id: u64, text: &str) {
        // Remove existing entry first so re-indexing replaces cleanly
        if self.doc_frequencies.contains_key(&doc_id) {
            self.remove_document(doc_id);
        }
        let tokens = Self::tokenize(text, self.stop_words.as_ref());
        let mut term_counts: HashMap<String, usize> = HashMap::new();

        for token in &tokens {
            *term_counts.entry(token.clone()).or_insert(0) += 1;
        }

        for token in &tokens {
            self.inverted_index
                .entry(token.clone())
                .or_default()
                .insert(doc_id);
        }

        self.doc_frequencies.insert(doc_id, term_counts);
        self.total_docs = self.doc_frequencies.len() as u64;
    }

    /// Remove a document from the index.
    ///
    /// This removes all term associations for the given document ID
    /// from both the inverted index and the term-frequency table.
    pub fn remove_document(&mut self, doc_id: u64) {
        if let Some(term_counts) = self.doc_frequencies.remove(&doc_id) {
            for term in term_counts.keys() {
                if let Some(docs) = self.inverted_index.get_mut(term) {
                    docs.remove(&doc_id);
                    if docs.is_empty() {
                        self.inverted_index.remove(term);
                    }
                }
            }
        }
        self.total_docs = self.doc_frequencies.len() as u64;
    }

    /// Search for documents matching the query.
    ///
    /// Returns a vector of `(doc_id, score)` pairs sorted by descending score.
    /// The score is computed using TF-IDF ranking.
    ///
    /// * `SearchMode::And` — only documents containing all query terms are returned.
    /// * `SearchMode::Or` — documents containing any query term are returned.
    /// * `SearchMode::Phrase` — only documents containing the exact query phrase
    ///   (in order) are returned. Phrase matching uses a simplified approach:
    ///   all phrase terms must appear in the document.
    pub fn search(&self, query: &str, mode: SearchMode) -> Vec<(u64, f64)> {
        let terms = Self::tokenize(query, self.stop_words.as_ref());
        if terms.is_empty() || self.total_docs == 0 {
            return Vec::new();
        }

        let candidate_docs = match &mode {
            SearchMode::And | SearchMode::Phrase => {
                let mut iter = terms.iter().filter_map(|t| self.inverted_index.get(t));
                let first = match iter.next() {
                    Some(set) => set.clone(),
                    None => return Vec::new(),
                };
                iter.fold(first, |acc, set| acc.intersection(set).copied().collect())
            }
            SearchMode::Or => {
                let mut union = HashSet::new();
                for term in &terms {
                    if let Some(docs) = self.inverted_index.get(term) {
                        union.extend(docs);
                    }
                }
                union
            }
        };

        let mut scored: Vec<(u64, f64)> = candidate_docs
            .iter()
            .map(|&doc_id| {
                let mut score = 0.0;
                for term in &terms {
                    score += self.tf_idf(term, doc_id);
                }
                (doc_id, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Compute the TF-IDF score for a term in a document.
    ///
    /// TF (term frequency) = log(1 + count) inside the document.
    /// IDF (inverse document frequency) = log(N / df), where N is the total
    /// number of documents and df is the number of documents containing the term.
    fn tf_idf(&self, term: &str, doc_id: u64) -> f64 {
        let tf = self
            .doc_frequencies
            .get(&doc_id)
            .and_then(|terms| terms.get(term))
            .map(|&count| (1.0 + count as f64).ln())
            .unwrap_or(0.0);

        let df = self
            .inverted_index
            .get(term)
            .map(|docs| docs.len() as f64)
            .unwrap_or(1.0);

        let idf = (self.total_docs as f64 / df).ln();

        tf * idf
    }

    /// Tokenize text into lowercase terms.
    ///
    /// Splits on whitespace and common punctuation, removes empty tokens,
    /// and optionally filters out stop words.
    fn tokenize(text: &str, stop_words: Option<&HashSet<String>>) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();

        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '\'' {
                current.push(ch.to_ascii_lowercase());
            } else {
                if !current.is_empty() {
                    if let Some(stop) = stop_words {
                        if !stop.contains(&current) {
                            tokens.push(current.clone());
                        }
                    } else {
                        tokens.push(current.clone());
                    }
                    current.clear();
                }
            }
        }
        if !current.is_empty() {
            if let Some(stop) = stop_words {
                if !stop.contains(&current) {
                    tokens.push(current);
                }
            } else {
                tokens.push(current);
            }
        }

        tokens
    }

    /// Return the total number of indexed documents.
    pub fn len(&self) -> u64 {
        self.total_docs
    }

    /// Return `true` if the index contains no documents.
    pub fn is_empty(&self) -> bool {
        self.total_docs == 0
    }

    /// Return the number of unique terms in the index.
    pub fn unique_term_count(&self) -> usize {
        self.inverted_index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_index() -> FullTextIndex {
        let mut idx = FullTextIndex::new();
        idx.index_document(1, "The quick brown fox jumps over the lazy dog");
        idx.index_document(2, "A quick brown fox is fast");
        idx.index_document(3, "The lazy dog sleeps all day");
        idx
    }

    #[test]
    fn test_index_and_search() {
        let idx = test_index();
        let results = idx.search("quick fox", SearchMode::And);
        assert!(!results.is_empty(), "expected matches for quick AND fox");
        let ids: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn test_remove_document() {
        let mut idx = test_index();
        assert_eq!(idx.len(), 3);
        idx.remove_document(2);
        assert_eq!(idx.len(), 2);
        let results = idx.search("quick", SearchMode::And);
        let ids: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(!ids.contains(&2));
    }

    #[test]
    fn test_and_mode() {
        let idx = test_index();
        let results = idx.search("quick fox", SearchMode::And);
        for (id, _) in &results {
            let doc = idx.doc_frequencies.get(id).unwrap();
            assert!(doc.contains_key("quick"), "doc {} missing 'quick'", id);
            assert!(doc.contains_key("fox"), "doc {} missing 'fox'", id);
        }
    }

    #[test]
    fn test_or_mode() {
        let idx = test_index();
        let results = idx.search("fast sleeps", SearchMode::Or);
        assert_eq!(results.len(), 2);
        let ids: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&2)); // "fast" in doc 2
        assert!(ids.contains(&3)); // "sleeps" in doc 3
    }

    #[test]
    fn test_no_matches() {
        let idx = test_index();
        let results = idx.search("xyznonexistent", SearchMode::And);
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_index() {
        let idx = FullTextIndex::new();
        let results = idx.search("anything", SearchMode::And);
        assert!(results.is_empty());
    }

    #[test]
    fn test_tokenize() {
        let tokens = FullTextIndex::tokenize("Hello, World! It's nice.", None);
        assert_eq!(tokens, vec!["hello", "world", "it's", "nice"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = FullTextIndex::tokenize("   ", None);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_stop_words() {
        let stop: HashSet<String> = ["the", "a", "is"].iter().map(|s| s.to_string()).collect();
        let tokens = FullTextIndex::tokenize("The cat is on a mat", Some(&stop));
        assert_eq!(tokens, vec!["cat", "on", "mat"]);
    }

    #[test]
    fn test_tf_idf_scoring() {
        let idx = test_index();
        let results = idx.search("lazy dog", SearchMode::And);
        assert_eq!(results.len(), 2);
        // Doc 1 has "lazy dog" in same doc, doc 3 has "lazy dog" too.
        // Both should be scored; the exact ordering depends on TF-IDF.
        let ids: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
    }

    #[test]
    fn test_phrase_mode() {
        let idx = test_index();
        let results = idx.search("lazy dog", SearchMode::Phrase);
        // Both doc 1 and doc 3 contain both terms
        assert!(!results.is_empty());
        let ids: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
    }

    #[test]
    fn test_with_stop_words_constructor() {
        let stop: HashSet<String> = ["the", "a", "an"].iter().map(|s| s.to_string()).collect();
        let mut idx = FullTextIndex::with_stop_words(stop);
        idx.index_document(1, "The cat and a dog");
        let terms: Vec<&String> = idx.doc_frequencies[&1].keys().collect();
        assert!(!terms.contains(&&"the".to_string()));
        assert!(!terms.contains(&&"a".to_string()));
        assert!(terms.contains(&&"cat".to_string()));
        assert!(terms.contains(&&"dog".to_string()));
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut idx = FullTextIndex::new();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
        idx.index_document(1, "hello world");
        assert!(!idx.is_empty());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn test_unique_term_count() {
        let mut idx = FullTextIndex::new();
        assert_eq!(idx.unique_term_count(), 0);
        idx.index_document(1, "hello world hello");
        assert_eq!(idx.unique_term_count(), 2);
        idx.index_document(2, "hello everyone");
        assert_eq!(idx.unique_term_count(), 3);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut idx = test_index();
        let len_before = idx.len();
        idx.remove_document(999);
        assert_eq!(idx.len(), len_before);
    }

    #[test]
    fn test_reindex_document() {
        let mut idx = FullTextIndex::new();
        idx.index_document(1, "hello world");
        assert_eq!(idx.len(), 1);
        // Re-indexing same doc replaces its terms
        idx.index_document(1, "foo bar");
        let results = idx.search("hello", SearchMode::And);
        assert!(results.is_empty());
        let results = idx.search("foo", SearchMode::And);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }
}
