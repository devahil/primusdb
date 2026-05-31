/*
 * PrimusDB Vector Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 *
 * Professional vector database engine with:
 *   - HNSW (Hierarchical Navigable Small World) graph index
 *   - IVF (Inverted File) with pre-assigned cluster lists
 *   - Flat (brute-force) search
 *   - Payload/metadata filtering (must/should/must_not, ranges, exists)
 *   - Scoring engine with cosine, dot, L2, RRF fusion, weighted scoring
 *   - Scalar Quantization (SQ8) and Binary Quantization (BQ)
 *   - RAG pipeline (document ingestion, chunking, retrieval)
 *   - Predictive analytics (k-means clustering, anomaly detection)
 *   - Metrics and observability
 */

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::info;

// ═══════════════════════════════════════════════════════════════════════════════
// Core Types
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Manhattan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexMethod {
    Flat,
    IVF { nlist: usize, nprobe: usize },
    HNSW { m: usize, ef_construction: usize, ef_search: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub method: IndexMethod,
    pub metric: DistanceMetric,
    pub dimension: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantizationType {
    None,
    Scalar8,   // SQ8: 4 bytes per float → 1 byte
    Binary,    // BQ: 32 floats per 32-bit signature
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoringMode {
    /// Use raw distance metric (lower is better for L2, higher for cosine/dot)
    Raw,
    /// Normalize all scores to [0, 1]
    Normalized,
    /// Reciprocal Rank Fusion for hybrid results
    RRF { k: usize },
    /// Weighted linear combination of multiple scores
    Weighted { weights: Vec<f32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub dimension: usize,
    pub metric: DistanceMetric,
    pub index_method: IndexMethod,
    pub quantization: QuantizationType,
    pub scoring_mode: ScoringMode,
    pub payload_schema: Option<serde_json::Value>,
    pub max_vectors: Option<usize>,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        CollectionConfig {
            dimension: 0,
            metric: DistanceMetric::Cosine,
            index_method: IndexMethod::Flat,
            quantization: QuantizationType::None,
            scoring_mode: ScoringMode::Normalized,
            payload_schema: None,
            max_vectors: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Payload Filter Engine
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOp {
    Eq(serde_json::Value),
    Ne(serde_json::Value),
    Gt(serde_json::Value),
    Gte(serde_json::Value),
    Lt(serde_json::Value),
    Lte(serde_json::Value),
    In(Vec<serde_json::Value>),
    Nin(Vec<serde_json::Value>),
    Exists(bool),
    Regex(String),
    And(Vec<PayloadFilter>),
    Or(Vec<PayloadFilter>),
    Not(Box<PayloadFilter>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadFilter {
    pub field: String,
    pub op: FilterOp,
}

impl PayloadFilter {
    pub fn matches(&self, data: &serde_json::Value) -> bool {
        match &self.op {
            FilterOp::And(filters) => filters.iter().all(|f| f.matches(data)),
            FilterOp::Or(filters) => filters.iter().any(|f| f.matches(data)),
            FilterOp::Not(filter) => !filter.matches(data),
            _ => {
                let val = data.get(&self.field);
                self.eval_field(val)
            }
        }
    }

    fn eval_field(&self, val: Option<&serde_json::Value>) -> bool {
        match &self.op {
            FilterOp::Eq(expected) => val.map_or(false, |v| v == expected),
            FilterOp::Ne(expected) => val.map_or(true, |v| v != expected),
            FilterOp::Gt(expected) => val.and_then(|v| compare_json_values(v, expected).map(|r| r > std::cmp::Ordering::Equal)).unwrap_or(false),
            FilterOp::Gte(expected) => val.and_then(|v| compare_json_values(v, expected).map(|r| r != std::cmp::Ordering::Less)).unwrap_or(false),
            FilterOp::Lt(expected) => val.and_then(|v| compare_json_values(v, expected).map(|r| r == std::cmp::Ordering::Less)).unwrap_or(false),
            FilterOp::Lte(expected) => val.and_then(|v| compare_json_values(v, expected).map(|r| r != std::cmp::Ordering::Greater)).unwrap_or(false),
            FilterOp::In(expected_list) => val.map_or(false, |v| expected_list.contains(v)),
            FilterOp::Nin(expected_list) => val.map_or(true, |v| !expected_list.contains(v)),
            FilterOp::Exists(should_exist) => val.is_some() == *should_exist,
            FilterOp::Regex(pattern) => {
                val.and_then(|v| v.as_str()).map_or(false, |s| {
                    regex::Regex::new(pattern).map_or(false, |re| re.is_match(s))
                })
            }
            _ => true,
        }
    }
}

fn compare_json_values(a: &serde_json::Value, b: &serde_json::Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (serde_json::Value::Number(an), serde_json::Value::Number(bn)) => {
            an.as_f64().partial_cmp(&bn.as_f64())
        }
        (serde_json::Value::String(as_), serde_json::Value::String(bs_)) => Some(as_.cmp(bs_)),
        _ => None,
    }
}

/// Parse conditions JSON into a list of PayloadFilters
fn parse_payload_filters(conditions: &serde_json::Value) -> Vec<PayloadFilter> {
    let mut filters = Vec::new();
    if let Some(obj) = conditions.as_object() {
        for (key, val) in obj {
            if key == "query_vector" || key == "vector" {
                continue;
            }
            if let Some(op_obj) = val.as_object() {
                for (op_key, op_val) in op_obj {
                    let filter_op = match op_key.as_str() {
                        "$eq" => Some(FilterOp::Eq(op_val.clone())),
                        "$ne" => Some(FilterOp::Ne(op_val.clone())),
                        "$gt" => Some(FilterOp::Gt(op_val.clone())),
                        "$gte" => Some(FilterOp::Gte(op_val.clone())),
                        "$lt" => Some(FilterOp::Lt(op_val.clone())),
                        "$lte" => Some(FilterOp::Lte(op_val.clone())),
                        "$in" => op_val.as_array().map(|arr| FilterOp::In(arr.clone())),
                        "$nin" => op_val.as_array().map(|arr| FilterOp::Nin(arr.clone())),
                        "$exists" => op_val.as_bool().map(|b| FilterOp::Exists(b)),
                        "$regex" => op_val.as_str().map(|s| FilterOp::Regex(s.to_string())),
                        _ => None,
                    };
                    if let Some(op) = filter_op {
                        filters.push(PayloadFilter { field: key.clone(), op });
                    }
                }
            } else {
                filters.push(PayloadFilter {
                    field: key.clone(),
                    op: FilterOp::Eq(val.clone()),
                });
            }
        }
    }
    filters
}

// ═══════════════════════════════════════════════════════════════════════════════
// Distance Functions (single source of truth)
// ═══════════════════════════════════════════════════════════════════════════════

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt()
}

pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
}

/// Compute similarity from distance based on metric type.
/// Returns a score where HIGHER is more similar.
fn compute_similarity(query: &[f32], vec: &[f32], metric: &DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Cosine => cosine_similarity(query, vec),
        DistanceMetric::Euclidean => -euclidean_distance(query, vec),
        DistanceMetric::DotProduct => dot_product(query, vec),
        DistanceMetric::Manhattan => -manhattan_distance(query, vec),
    }
}

/// Compute distance (lower = more similar)
fn compute_distance(query: &[f32], vec: &[f32], metric: &DistanceMetric) -> f32 {
    match metric {
        DistanceMetric::Cosine => 1.0 - cosine_similarity(query, vec),
        DistanceMetric::Euclidean => euclidean_distance(query, vec),
        DistanceMetric::DotProduct => -dot_product(query, vec),
        DistanceMetric::Manhattan => manhattan_distance(query, vec),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scalar Quantization (SQ8)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantizer {
    /// Per-dimension min values
    min: Vec<f32>,
    /// Per-dimension (max - min) range
    range: Vec<f32>,
    dimension: usize,
}

impl ScalarQuantizer {
    pub fn train(vectors: &[Vec<f32>]) -> Self {
        let dim = vectors[0].len();
        let mut min = vec![f32::MAX; dim];
        let mut max = vec![f32::MIN; dim];
        for v in vectors {
            for d in 0..dim {
                min[d] = min[d].min(v[d]);
                max[d] = max[d].max(v[d]);
            }
        }
        let range: Vec<f32> = min.iter().zip(max.iter()).map(|(mn, mx)| (mx - mn).max(1e-10)).collect();
        ScalarQuantizer { min, range, dimension: dim }
    }

    pub fn quantize(&self, vec: &[f32]) -> Vec<u8> {
        vec.iter().enumerate().map(|(d, &v)| {
            let normalized = (v - self.min[d]) / self.range[d];
            (normalized * 255.0).round().clamp(0.0, 255.0) as u8
        }).collect()
    }

    pub fn dequantize(&self, bytes: &[u8]) -> Vec<f32> {
        bytes.iter().enumerate().map(|(d, &b)| {
            self.min[d] + (b as f32 / 255.0) * self.range[d]
        }).collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Binary Quantization (BQ)
// ═══════════════════════════════════════════════════════════════════════════════

/// Compute a binary signature: 1 if element ≥ median of dimension, else 0
fn binary_quantize(vec: &[f32]) -> Vec<u64> {
    let median = {
        let mut sorted = vec.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sorted[sorted.len() / 2]
    };
    let words = (vec.len() + 63) / 64;
    let mut sig = vec![0u64; words];
    for (i, &v) in vec.iter().enumerate() {
        if v >= median {
            sig[i / 64] |= 1u64 << (i % 64);
        }
    }
    sig
}

#[allow(dead_code)]
fn hamming_distance(a: &[u64], b: &[u64]) -> u32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum()
}

// ═══════════════════════════════════════════════════════════════════════════════
// HNSW Index — Real Hierarchical Navigable Small World Graph
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswNode {
    /// Vector id (string key)
    id: String,
    /// Level (layer) of the node in the graph
    level: usize,
    /// Neighbors at each level (level → list of neighbor ids)
    neighbors: HashMap<usize, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswIndex {
    metric: DistanceMetric,
    m: usize,               // number of bi-directional connections per layer
    ef_construction: usize, // ef used during construction
    ef_search: usize,       // ef used during search
    ml: f32,                // level multiplier (1/ln(m))
    max_level: usize,
    enter_point: Option<String>,
    nodes: HashMap<String, HnswNode>,
    vectors: HashMap<String, Vec<f32>>,
}

impl HnswIndex {
    pub fn new(metric: DistanceMetric, m: usize, ef_construction: usize, ef_search: usize) -> Self {
        HnswIndex {
            metric,
            m,
            ef_construction,
            ef_search,
            ml: 1.0 / (m as f32).ln(),
            max_level: 0,
            enter_point: None,
            nodes: HashMap::new(),
            vectors: HashMap::new(),
        }
    }

    fn random_level(&self) -> usize {
        let mut hasher = DefaultHasher::new();
        self.nodes.len().hash(&mut hasher);
        let r = (hasher.finish() as f32) / (u64::MAX as f32);
        (-r.ln() * self.ml).max(0.0) as usize
    }

    pub fn insert(&mut self, id: &str, vector: Vec<f32>) {
        if self.nodes.contains_key(id) {
            return; // already exists
        }

        let level = self.random_level();
        let mut node = HnswNode {
            id: id.to_string(),
            level,
            neighbors: HashMap::new(),
        };

        // Initialize empty neighbor lists for levels 0..=level
        for l in 0..=level {
            node.neighbors.insert(l, Vec::new());
        }

        self.vectors.insert(id.to_string(), vector);

        if self.enter_point.is_none() {
            self.enter_point = Some(id.to_string());
            self.max_level = level;
            self.nodes.insert(id.to_string(), node);
            return;
        }

        let curr_ep = self.enter_point.clone().unwrap();

        // Phase 1: Traverse from top level to level+1 (greedy)
        let mut curr = curr_ep.clone();
        for lc in (level + 1..=self.max_level).rev() {
            let changed = self.search_layer_greedy(&curr, id, lc);
            if let Some(c) = changed { curr = c; }
        }

        // Phase 2: Descend from min(level, max_level) down to 0
        for lc in (0..=level.min(self.max_level)).rev() {
            let candidates = self.search_layer(id, lc, self.ef_construction);
            let neighbors = self.select_neighbors(candidates, lc);
            // Connect the new node to its neighbors
            let neigh_ids: Vec<String> = neighbors.iter().map(|n| n.0.clone()).collect();
            node.neighbors.insert(lc, neigh_ids.clone());

            // Connect neighbors back (bi-directional)
            // Phase 1: collect neighbor data without mutable self access
            let mut neighbor_updates: Vec<(String, Option<Vec<String>>)> = Vec::new();
            let m_val = self.m;
            let qv = self.vectors[id].clone();
            for neighbor_id in &neigh_ids {
                let current_len = self.nodes.get(neighbor_id)
                    .and_then(|n| n.neighbors.get(&lc))
                    .map(|n| n.len())
                    .unwrap_or(0);
                if current_len > m_val * 2 {
                    let n_shrink: Vec<(String, f32)> = self.nodes.get(neighbor_id)
                        .and_then(|n| n.neighbors.get(&lc))
                        .map(|neighbors| {
                            neighbors.iter().map(|nid| {
                                let v = &self.vectors[nid];
                                let score = compute_similarity(&qv, v, &self.metric);
                                (nid.clone(), score)
                            }).collect()
                        }).unwrap_or_default();
                    let shrunk = self.select_neighbors(n_shrink, lc);
                    let new_neighbors: Vec<String> = shrunk.into_iter().map(|(nid, _)| nid).collect();
                    neighbor_updates.push((neighbor_id.clone(), Some(new_neighbors)));
                } else {
                    neighbor_updates.push((neighbor_id.clone(), None));
                }
            }
            // Phase 2: apply updates with exclusive mutable access
            for (nid, replacement) in neighbor_updates {
                if let Some(neighbor_node) = self.nodes.get_mut(&nid) {
                    let n_neighbors = neighbor_node.neighbors.entry(lc).or_default();
                    if let Some(new_list) = replacement {
                        n_neighbors.clear();
                        for new_nid in new_list {
                            n_neighbors.push(new_nid);
                        }
                    } else {
                        n_neighbors.push(id.to_string());
                    }
                }
            }
        }

        // Update enter point if needed
        if level > self.max_level {
            self.max_level = level;
            self.enter_point = Some(id.to_string());
        }

        self.nodes.insert(id.to_string(), node);
    }

    fn search_layer_greedy(&self, entry: &str, query_id: &str, level: usize) -> Option<String> {
        let qv = self.vectors.get(query_id)?;
        let mut best = entry.to_string();
        let mut best_dist = compute_distance(qv, self.vectors.get(&best)?, &self.metric);

        loop {
            let mut improved = false;
            if let Some(neighbors) = self.nodes.get(&best).and_then(|n| n.neighbors.get(&level)) {
                for neighbor in neighbors {
                    if let Some(nv) = self.vectors.get(neighbor) {
                        let d = compute_distance(qv, nv, &self.metric);
                        if d < best_dist {
                            best_dist = d;
                            best = neighbor.clone();
                            improved = true;
                        }
                    }
                }
            }
            if !improved { break; }
        }
        Some(best)
    }

    fn search_layer(&self, query_id: &str, level: usize, ef: usize) -> Vec<(String, f32)> {
        let qv = match self.vectors.get(query_id) {
            Some(v) => v,
            None => return vec![],
        };

        let mut visited = HashSet::new();
        let mut candidates: Vec<(f32, String)> = Vec::new(); // (distance, id)
        let mut results: Vec<(f32, String)> = Vec::new();

        let ep = match &self.enter_point {
            Some(ep) => ep.clone(),
            None => return vec![],
        };

        // Initialize with enter point
        if let Some(epv) = self.vectors.get(&ep) {
            let d = compute_distance(qv, epv, &self.metric);
            candidates.push((d, ep.clone()));
            results.push((d, ep.clone()));
            visited.insert(ep.clone());
        }

        while !candidates.is_empty() {
            // Process closest candidate first
            candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let (dist, id) = candidates.remove(0);

            // Check distance threshold: if this candidate is farther than the farthest result, stop
            if results.len() >= ef && dist > results.last().map(|(d, _)| *d).unwrap_or(f32::MAX) {
                break;
            }

            if let Some(neighbors) = self.nodes.get(&id).and_then(|n| n.neighbors.get(&level)) {
                for neighbor in neighbors {
                    if visited.insert(neighbor.clone()) {
                        if let Some(nv) = self.vectors.get(neighbor) {
                            let d = compute_distance(qv, nv, &self.metric);
                            candidates.push((d, neighbor.clone()));
                            results.push((d, neighbor.clone()));
                        }
                    }
                }
            }

            // Keep top ef results (sorted by distance ascending = closest first)
            results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(ef);
        }

        results.into_iter().map(|(d, id)| (id, -d)).collect()
    }

    fn select_neighbors(&self, candidates: Vec<(String, f32)>, _level: usize) -> Vec<(String, f32)> {
        let mut sorted: Vec<(String, f32)> = candidates;
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(self.m);
        sorted
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.enter_point.is_none() || self.nodes.is_empty() {
            return vec![];
        }

        let mut visited = HashSet::new();
        let mut candidates: Vec<(f32, String)> = Vec::new();
        let mut results: Vec<(f32, String)> = Vec::new();

        let ep = self.enter_point.as_ref().unwrap();

        if let Some(epv) = self.vectors.get(ep) {
            let d = compute_distance(query, epv, &self.metric);
            candidates.push((d, ep.clone()));
            results.push((d, ep.clone()));
            visited.insert(ep.clone());
        }

        let ef = std::cmp::max(self.ef_search, top_k);

        // Search from top level down
        for level in (1..=self.max_level).rev() {
            let mut curr = ep.clone();
            let mut curr_dist = compute_distance(query, self.vectors.get(&curr).unwrap(), &self.metric);
            loop {
                let mut improved = false;
                if let Some(neighbors) = self.nodes.get(&curr).and_then(|n| n.neighbors.get(&level)) {
                    for neighbor in neighbors {
                        if let Some(nv) = self.vectors.get(neighbor) {
                            let d = compute_distance(query, nv, &self.metric);
                            if d < curr_dist {
                                curr_dist = d;
                                curr = neighbor.clone();
                                improved = true;
                            }
                        }
                    }
                }
                if !improved { break; }
            }
        }

        // Search at level 0 with ef candidates
        while !candidates.is_empty() {
            candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let (dist, id) = candidates.remove(0);

            if results.len() >= ef && dist > results.last().map(|(d, _)| *d).unwrap_or(f32::MAX) {
                break;
            }

            if let Some(neighbors) = self.nodes.get(&id).and_then(|n| n.neighbors.get(&0)) {
                for neighbor in neighbors {
                    if visited.insert(neighbor.clone()) {
                        if let Some(nv) = self.vectors.get(neighbor) {
                            let d = compute_distance(query, nv, &self.metric);
                            candidates.push((d, neighbor.clone()));
                            results.push((d, neighbor.clone()));
                        }
                    }
                }
            }

            results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(ef);
        }

        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results.into_iter().map(|(d, id)| (id, -d)).collect()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// IVF Index — Optimized with Pre-Assigned Cluster Lists
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfIndex {
    centroids: Vec<Vec<f32>>,
    /// Pre-assigned list: cluster_index → list of (vector_id, vector)
    inverted_lists: Vec<Vec<(String, Vec<f32>)>>,
    nprobe: usize,
    metric: DistanceMetric,
}

impl IvfIndex {
    pub fn build(vectors: &[(String, Vec<f32>)], nlist: usize, nprobe: usize, metric: &DistanceMetric) -> Self {
        let n = vectors.len();
        if n == 0 || nlist == 0 {
            return IvfIndex {
                centroids: vec![],
                inverted_lists: vec![],
                nprobe,
                metric: metric.clone(),
            };
        }

        let k = nlist.min(n);
        let dim = vectors[0].1.len();

        // K-means++ initialization
        let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);
        let seed = (n as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let first_idx = (seed % n as u64) as usize;
        centroids.push(vectors[first_idx].1.clone());

        let mut rng_state = seed;
        while centroids.len() < k {
            let dists: Vec<f32> = vectors.iter().map(|(_, v)| {
                centroids.iter().map(|c| euclidean_distance(v, c)).fold(f32::MAX, f32::min).powi(2)
            }).collect();
            let sum_d: f32 = dists.iter().sum();
            if sum_d == 0.0 { break; }
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let threshold = (rng_state as f32) / (u64::MAX as f32) * sum_d;
            let mut cumulative = 0.0f32;
            for (i, d) in dists.iter().enumerate() {
                cumulative += d;
                if cumulative >= threshold {
                    centroids.push(vectors[i].1.clone());
                    break;
                }
            }
        }

        // 10 iterations of K-means
        for _iteration in 0..10 {
            let mut assignments: Vec<Vec<usize>> = (0..centroids.len()).map(|_| vec![]).collect();
            for (i, v) in vectors.iter().enumerate() {
                let mut best = 0usize;
                let mut best_d = f32::MAX;
                for (j, c) in centroids.iter().enumerate() {
                    let d = match metric {
                        DistanceMetric::Cosine => 1.0 - cosine_similarity(&v.1, c),
                        DistanceMetric::Euclidean => euclidean_distance(&v.1, c),
                        DistanceMetric::DotProduct => -dot_product(&v.1, c),
                        DistanceMetric::Manhattan => manhattan_distance(&v.1, c),
                    };
                    if d < best_d {
                        best_d = d;
                        best = j;
                    }
                }
                assignments[best].push(i);
            }
            for (j, members) in assignments.iter().enumerate() {
                if members.is_empty() { continue; }
                let mut new_centroid = vec![0.0f32; dim];
                for &mi in members {
                    for d in 0..dim {
                        new_centroid[d] += vectors[mi].1[d];
                    }
                }
                let inv = members.len() as f32;
                for d in 0..dim {
                    new_centroid[d] /= inv;
                }
                centroids[j] = new_centroid;
            }
        }

        // Build inverted lists: assign each vector to nearest centroid
        let mut inverted_lists: Vec<Vec<(String, Vec<f32>)>> = (0..centroids.len()).map(|_| vec![]).collect();
        for (id, v) in vectors {
            let mut best = 0usize;
            let mut best_d = f32::MAX;
            for (j, c) in centroids.iter().enumerate() {
                let d = match metric {
                    DistanceMetric::Cosine => 1.0 - cosine_similarity(v, c),
                    DistanceMetric::Euclidean => euclidean_distance(v, c),
                    DistanceMetric::DotProduct => -dot_product(v, c),
                    DistanceMetric::Manhattan => manhattan_distance(v, c),
                };
                if d < best_d {
                    best_d = d;
                    best = j;
                }
            }
            inverted_lists[best].push((id.clone(), v.clone()));
        }

        IvfIndex {
            centroids,
            inverted_lists,
            nprobe,
            metric: metric.clone(),
        }
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.centroids.is_empty() || self.inverted_lists.is_empty() {
            return vec![];
        }

        // Find nearest nprobe centroids
        let mut centroid_dists: Vec<(usize, f32)> = self.centroids.iter().enumerate().map(|(i, c)| {
            let d = compute_distance(query, c, &self.metric);
            (i, d)
        }).collect();
        centroid_dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let probe_count = self.nprobe.min(self.centroids.len());
        let mut candidates: Vec<(String, f32)> = Vec::new();

        for &(ci, _) in centroid_dists.iter().take(probe_count) {
            for (id, v) in &self.inverted_lists[ci] {
                let sim = compute_similarity(query, v, &self.metric);
                candidates.push((id.clone(), sim));
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(top_k);
        candidates
    }

    pub fn len(&self) -> usize {
        self.inverted_lists.iter().map(|l| l.len()).sum()
    }

    pub fn brute_force_search(&self, query: &[f32], vectors: &[(String, Vec<f32>)], top_k: usize) -> Vec<(String, f32)> {
        let mut results: Vec<(String, f32)> = vectors.iter().map(|(id, v)| {
            let sim = compute_similarity(query, v, &self.metric);
            (id.clone(), sim)
        }).collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scoring Engine
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    pub id: String,
    pub score: f32,
    pub scores: Vec<f32>,  // per-query scores for fusion
}

pub fn reciprocal_rank_fusion(results: Vec<Vec<ScoredResult>>, k: usize, top_n: usize) -> Vec<ScoredResult> {
    let mut combined: HashMap<String, (f32, Vec<f32>)> = HashMap::new();
    let num_lists = results.len();
    for list_idx in 0..num_lists {
        for (rank, result) in results[list_idx].iter().enumerate() {
            let entry = combined.entry(result.id.clone()).or_insert_with(|| (0.0, vec![0.0; num_lists]));
            entry.0 += 1.0 / (k as f32 + rank as f32);
            entry.1[list_idx] = result.score;
        }
    }

    let mut fused: Vec<ScoredResult> = combined.into_iter().map(|(id, (score, scores))| {
        ScoredResult { id, score, scores }
    }).collect();

    fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    fused.truncate(top_n);
    fused
}

pub fn weighted_fusion(results: Vec<Vec<ScoredResult>>, weights: &[f32], top_n: usize) -> Vec<ScoredResult> {
    let mut combined: HashMap<String, (f32, Vec<f32>)> = HashMap::new();
    let num_lists = results.len();
    for list_idx in 0..num_lists {
        if list_idx >= weights.len() { break; }
        let w = weights[list_idx];
        for result in &results[list_idx] {
            let entry = combined.entry(result.id.clone()).or_insert_with(|| (0.0, vec![0.0; num_lists]));
            entry.0 += w * result.score;
            entry.1[list_idx] = result.score;
        }
    }

    let mut fused: Vec<ScoredResult> = combined.into_iter().map(|(id, (score, scores))| {
        ScoredResult { id, score, scores }
    }).collect();

    fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    fused.truncate(top_n);
    fused
}

// ═══════════════════════════════════════════════════════════════════════════════
// Predictive Analytics
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResult {
    pub cluster_id: usize,
    pub centroid: Vec<f32>,
    pub size: usize,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub id: String,
    pub score: f32,
    pub is_anomaly: bool,
}

pub fn kmeans_clustering(
    vectors: &[(String, Vec<f32>)],
    k: usize,
    iterations: usize,
) -> Vec<ClusterResult> {
    let n = vectors.len();
    if n == 0 || k == 0 { return vec![]; }
    let k = k.min(n);
    let dim = vectors[0].1.len();

    let mut centroids: Vec<Vec<f32>> = Vec::new();
    let mut taken = HashSet::new();
    let mut rng_state = (k as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    while centroids.len() < k {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let idx = (rng_state % n as u64) as usize;
        if taken.insert(idx) {
            centroids.push(vectors[idx].1.clone());
        }
    }

    let mut assignments = vec![0usize; n];
    for _iter in 0..iterations {
        for (i, v) in vectors.iter().enumerate() {
            let mut best = 0;
            let mut best_d = f32::MAX;
            for (j, c) in centroids.iter().enumerate() {
                let d = euclidean_distance(&v.1, c);
                if d < best_d { best_d = d; best = j; }
            }
            assignments[i] = best;
        }
        for j in 0..k {
            let members: Vec<usize> = assignments.iter().enumerate().filter(|(_, &a)| a == j).map(|(i, _)| i).collect();
            if members.is_empty() { continue; }
            let mut new_c = vec![0.0f32; dim];
            for &mi in &members {
                for d in 0..dim {
                    new_c[d] += vectors[mi].1[d];
                }
            }
            let inv = members.len() as f32;
            for d in 0..dim { new_c[d] /= inv; }
            centroids[j] = new_c;
        }
    }

    let mut clusters: Vec<ClusterResult> = Vec::new();
    for j in 0..k {
        let members: Vec<usize> = assignments.iter().enumerate().filter(|(_, &a)| a == j).map(|(i, _)| i).collect();
        let member_ids: Vec<String> = members.iter().map(|&mi| vectors[mi].0.clone()).collect();
        clusters.push(ClusterResult {
            cluster_id: j,
            centroid: centroids[j].clone(),
            size: members.len(),
            members: member_ids,
        });
    }
    clusters
}

pub fn detect_anomalies(
    vectors: &[(String, Vec<f32>)],
    threshold: f32,
) -> Vec<AnomalyResult> {
    let n = vectors.len();
    if n == 0 { return vec![]; }

    // Compute global centroid
    let dim = vectors[0].1.len();
    let mut centroid = vec![0.0f32; dim];
    for (_, v) in vectors {
        for d in 0..dim {
            centroid[d] += v[d];
        }
    }
    let inv = n as f32;
    for d in 0..dim { centroid[d] /= inv; }

    // Compute mean distance to centroid
    let distances: Vec<f32> = vectors.iter().map(|(_, v)| euclidean_distance(v, &centroid)).collect();
    let mean_dist: f32 = distances.iter().sum::<f32>() / n as f32;

    // Compute std deviation
    let variance: f32 = distances.iter().map(|d| (d - mean_dist).powi(2)).sum::<f32>() / n as f32;
    let std_dev = variance.sqrt();

    // Flag anomalies: distance > mean + threshold * std_dev
    let cutoff = mean_dist + threshold * std_dev;
    vectors.iter().zip(distances.iter()).map(|((id, _), &dist)| {
        AnomalyResult {
            id: id.clone(),
            score: dist,
            is_anomaly: dist > cutoff,
        }
    }).collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// RAG Subsystem
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub document_id: String,
    pub text: String,
    pub chunk_index: usize,
    pub metadata: HashMap<String, String>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub source: String,
    pub text: String,
    pub metadata: HashMap<String, String>,
    pub checksum: String,
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkingStrategy {
    Fixed { chunk_size: usize, overlap: usize },
    Recursive { chunk_size: usize, overlap: usize, separators: Vec<String> },
    SlidingWindow { chunk_size: usize, overlap: usize },
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        ChunkingStrategy::Fixed { chunk_size: 512, overlap: 64 }
    }
}

pub fn sha256_checksum(text: &str) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(text.as_bytes());
    format!("{:x}", hash)
}

pub fn chunk_document(
    doc_id: &str,
    title: &str,
    source: &str,
    text: &str,
    strategy: &ChunkingStrategy,
    metadata: HashMap<String, String>,
) -> Document {
    let checksum = sha256_checksum(text);
    let chunks = match strategy {
        ChunkingStrategy::Fixed { chunk_size, overlap } => {
            let step = chunk_size.saturating_sub(*overlap).max(1);
            text.as_bytes().chunks(step).enumerate().map(|(i, chunk_bytes)| {
                let chunk_text = String::from_utf8_lossy(chunk_bytes).to_string();
                let chunk_text = if chunk_text.len() > *chunk_size {
                    chunk_text.chars().take(*chunk_size).collect()
                } else {
                    chunk_text
                };
                Chunk {
                    id: format!("{}_chunk_{}", doc_id, i),
                    document_id: doc_id.to_string(),
                    text: chunk_text,
                    chunk_index: i,
                    metadata: metadata.clone(),
                    embedding: None,
                }
            }).collect()
        }
        ChunkingStrategy::Recursive { chunk_size, overlap, separators } => {
            let mut chunks = Vec::new();
            let mut remaining = text.to_string();
            let mut idx = 0;
            while !remaining.is_empty() {
                let (chunk_text, new_remaining) = if remaining.len() > *chunk_size {
                    let slice = &remaining[..*chunk_size];
                    let split_pos = separators.iter()
                        .filter_map(|sep| slice.rfind(sep))
                        .max()
                        .unwrap_or(0);

                    if split_pos > 0 {
                        let chunk = remaining[..split_pos].to_string();
                        let overlap_start = if split_pos > *overlap { split_pos - overlap } else { 0 };
                        let new_remaining = remaining[overlap_start..].to_string();
                        // Avoid infinite loop if no progress
                        if new_remaining.len() >= remaining.len() {
                            let chunk = remaining[..*chunk_size].to_string();
                            let new_remaining = remaining[*chunk_size..].to_string();
                            (chunk, new_remaining)
                        } else {
                            (chunk, new_remaining)
                        }
                    } else {
                        let chunk = remaining[..*chunk_size].to_string();
                        let new_remaining = remaining[*chunk_size..].to_string();
                        (chunk, new_remaining)
                    }
                } else {
                    let chunk = remaining.clone();
                    let new_remaining = String::new();
                    (chunk, new_remaining)
                };
                if !chunk_text.is_empty() {
                    chunks.push(Chunk {
                        id: format!("{}_chunk_{}", doc_id, idx),
                        document_id: doc_id.to_string(),
                        text: chunk_text,
                        chunk_index: idx,
                        metadata: metadata.clone(),
                        embedding: None,
                    });
                }
                idx += 1;
                remaining = new_remaining;
                if remaining.len() == text.len() && idx > 100 { break; } // safety
            }
            chunks
        }
        ChunkingStrategy::SlidingWindow { chunk_size, overlap } => {
            let step = chunk_size.saturating_sub(*overlap).max(1);
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            let mut idx = 0;
            let mut chunks = Vec::new();
            while i < chars.len() {
                let end = (i + chunk_size).min(chars.len());
                let chunk_text: String = chars[i..end].iter().collect();
                chunks.push(Chunk {
                    id: format!("{}_chunk_{}", doc_id, idx),
                    document_id: doc_id.to_string(),
                    text: chunk_text,
                    chunk_index: idx,
                    metadata: metadata.clone(),
                    embedding: None,
                });
                i += step;
                idx += 1;
            }
            chunks
        }
    };

    Document {
        id: doc_id.to_string(),
        title: title.to_string(),
        source: source.to_string(),
        text: text.to_string(),
        metadata,
        checksum,
        chunks,
    }
}

pub fn rag_retrieve_similar_chunks(
    query: &[f32],
    documents: &[Document],
    top_k: usize,
    metric: &DistanceMetric,
) -> Vec<(Chunk, f32)> {
    let mut scored_chunks: Vec<(Chunk, f32)> = Vec::new();
    for doc in documents {
        for chunk in &doc.chunks {
            if let Some(ref emb) = chunk.embedding {
                let sim = compute_similarity(query, emb, metric);
                scored_chunks.push((chunk.clone(), sim));
            }
        }
    }
    scored_chunks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored_chunks.truncate(top_k);
    scored_chunks
}

// ═══════════════════════════════════════════════════════════════════════════════
// Metrics Collector
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct VectorMetrics {
    pub query_count: AtomicU64,
    pub total_query_latency_ms: AtomicU64,
    pub index_build_count: AtomicU64,
    pub total_index_build_ms: AtomicU64,
    pub vector_count: AtomicU64,
    pub deleted_vector_count: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl VectorMetrics {
    pub fn new() -> Self {
        VectorMetrics {
            query_count: AtomicU64::new(0),
            total_query_latency_ms: AtomicU64::new(0),
            index_build_count: AtomicU64::new(0),
            total_index_build_ms: AtomicU64::new(0),
            vector_count: AtomicU64::new(0),
            deleted_vector_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    pub fn record_query(&self, latency_ms: f64) {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.total_query_latency_ms.fetch_add((latency_ms * 1000.0) as u64, Ordering::Relaxed);
    }

    pub fn record_index_build(&self, duration_ms: f64) {
        self.index_build_count.fetch_add(1, Ordering::Relaxed);
        self.total_index_build_ms.fetch_add((duration_ms * 1000.0) as u64, Ordering::Relaxed);
    }

    pub fn avg_query_latency_ms(&self) -> f64 {
        let count = self.query_count.load(Ordering::Relaxed);
        if count == 0 { return 0.0; }
        let total = self.total_query_latency_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        total / count as f64
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "query_count": self.query_count.load(Ordering::Relaxed),
            "avg_query_latency_ms": self.avg_query_latency_ms(),
            "index_build_count": self.index_build_count.load(Ordering::Relaxed),
            "vector_count": self.vector_count.load(Ordering::Relaxed),
            "deleted_vector_count": self.deleted_vector_count.load(Ordering::Relaxed),
            "cache_hits": self.cache_hits.load(Ordering::Relaxed),
            "cache_misses": self.cache_misses.load(Ordering::Relaxed),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VectorEngine — Main Storage Engine Implementation
// ═══════════════════════════════════════════════════════════════════════════════

pub struct VectorEngine {
    #[allow(dead_code)]
    config: PrimusDBConfig,
    db: sled::Db,
    #[allow(dead_code)]
    index_method: Arc<std::sync::RwLock<HashMap<String, IndexMethod>>>,
    collection_configs: Arc<std::sync::RwLock<HashMap<String, CollectionConfig>>>,
    hnsw_indices: Arc<Mutex<HashMap<String, HnswIndex>>>,
    ivf_indices: Arc<Mutex<HashMap<String, IvfIndex>>>,
    id_counter: AtomicU64,
    metrics: Arc<VectorMetrics>,
    /// RAG document storage
    rag_documents: Arc<std::sync::RwLock<HashMap<String, Vec<Document>>>>,
}

impl VectorEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let db_path = format!("{}/vector", config.storage.data_dir);
        let db = sled::open(&db_path)?;

        Ok(VectorEngine {
            config: config.clone(),
            db,
            index_method: Arc::new(std::sync::RwLock::new(HashMap::new())),
            collection_configs: Arc::new(std::sync::RwLock::new(HashMap::new())),
            hnsw_indices: Arc::new(Mutex::new(HashMap::new())),
            ivf_indices: Arc::new(Mutex::new(HashMap::new())),
            id_counter: AtomicU64::new(1),
            metrics: Arc::new(VectorMetrics::new()),
            rag_documents: Arc::new(std::sync::RwLock::new(HashMap::new())),
        })
    }

    pub fn metrics(&self) -> &VectorMetrics {
        &self.metrics
    }

    /// Build/rebuild the HNSW index for a collection
    pub fn build_hnsw_index(&self, table: &str, m: usize, ef_construction: usize, ef_search: usize) -> Result<()> {
        let start = Instant::now();
        let vectors = self.load_all_vectors(table)?;
        let config = self.collection_configs.read().unwrap().get(table).cloned().unwrap_or_default();
        let metric = config.metric;

        {
            let mut indices = self.hnsw_indices.lock().unwrap();
            indices.insert(table.to_string(),
                HnswIndex::new(metric.clone(), m, ef_construction, ef_search));
        }

        {
            let mut new_idx = HnswIndex::new(metric.clone(), m, ef_construction, ef_search);
            for (id, v) in &vectors {
                new_idx.insert(id, v.clone());
            }
            self.hnsw_indices.lock().unwrap().insert(table.to_string(), new_idx);
        }

        self.metrics.record_index_build(start.elapsed().as_secs_f64() * 1000.0);
        info!("HNSW index built for {} with {} vectors (m={}, ef={})", table, vectors.len(), m, ef_construction);
        Ok(())
    }

    /// Build/rebuild the IVF index for a collection
    pub fn build_ivf_index(&self, table: &str, nlist: usize, nprobe: usize) -> Result<()> {
        let start = Instant::now();
        let vectors = self.load_all_vectors(table)?;
        let config = self.collection_configs.read().unwrap().get(table).cloned().unwrap_or_default();

        if vectors.len() < 2 {
            return Ok(());
        }

        let index = IvfIndex::build(&vectors, nlist, nprobe, &config.metric);
        {
            let mut indices = self.ivf_indices.lock().unwrap();
            indices.insert(table.to_string(), index);
        }

        // Also persist IVF index to sled for durability
        let index_key = format!("ivf_index:{}", table);
        if let Some(index) = self.ivf_indices.lock().unwrap().get(table).cloned() {
            if let Ok(serialized) = bincode::serialize(&index) {
                let _ = self.db.insert(index_key.as_bytes(), serialized);
                let _ = self.db.flush();
            }
        }

        self.metrics.record_index_build(start.elapsed().as_secs_f64() * 1000.0);
        info!("IVF index built for {} with {} vectors (nlist={}, nprobe={})", table, vectors.len(), nlist, nprobe);
        Ok(())
    }

    fn load_all_vectors(&self, table: &str) -> Result<Vec<(String, Vec<f32>)>> {
        let table_key = format!("table:{}", table);
        let tree = self.db.open_tree(table_key)?;
        let mut vectors = Vec::new();
        for item in tree.iter() {
            let (key, value) = item?;
            let data: serde_json::Value = serde_json::from_slice(&value)?;
            if let Some(vec_array) = data.get("vector").and_then(|v| v.as_array()) {
                let v: Vec<f32> = vec_array.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
                if !v.is_empty() {
                    // Convert u64 key bytes to string id for consistency with select()
                    let id = if key.len() == 8 {
                        u64::from_be_bytes(key.as_ref().try_into().unwrap_or(0u64.to_be_bytes())).to_string()
                    } else {
                        String::from_utf8_lossy(&key.to_vec()).to_string()
                    };
                    vectors.push((id, v));
                }
            }
        }
        Ok(vectors)
    }

    #[allow(dead_code)]
    fn load_all_records(&self, table: &str) -> Result<Vec<(Vec<u8>, u64, serde_json::Value)>> {
        let table_key = format!("table:{}", table);
        let tree = self.db.open_tree(table_key)?;
        let mut records = Vec::new();
        for item in tree.iter() {
            let (key, value) = item?;
            let id = if key.len() == 8 {
                u64::from_be_bytes(key.as_ref().try_into().unwrap_or(0u64.to_be_bytes()))
            } else {
                String::from_utf8_lossy(&key.to_vec()).parse::<u64>().unwrap_or(0)
            };
            let data: serde_json::Value = serde_json::from_slice(&value)?;
            records.push((key.to_vec(), id, data));
        }
        Ok(records)
    }

    fn matches_conditions(data: &serde_json::Value, conditions: &serde_json::Value) -> bool {
        if conditions.is_null() || conditions.as_object().map_or(true, |o| o.is_empty()) {
            return true;
        }
        if conditions.get("query_vector").is_some() || conditions.get("vector").is_some() {
            // Vector search conditions - handled separately in select()
            return true;
        }
        // Check for operator-based filters
        let filters = parse_payload_filters(conditions);
        if !filters.is_empty() {
            return filters.iter().all(|f| f.matches(data));
        }
        // Legacy exact-match fallback
        if let Some(obj) = conditions.as_object() {
            for (key, cond_val) in obj {
                if key == "query_vector" || key == "vector" || key == "limit" { continue; }
                match data.get(key) {
                    Some(data_val) => { if data_val != cond_val { return false; } }
                    None => return false,
                }
            }
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn generate_id(&self) -> u64 {
        self.id_counter.fetch_add(1, Ordering::SeqCst)
    }

    // ── Set collection config ──
    pub fn set_collection_config(&self, table: &str, config: CollectionConfig) {
        self.collection_configs.write().unwrap().insert(table.to_string(), config);
    }

    pub fn get_collection_config(&self, table: &str) -> Option<CollectionConfig> {
        self.collection_configs.read().unwrap().get(table).cloned()
    }

    // ── RAG Operations ──
    pub fn rag_ingest_document(
        &self,
        collection: &str,
        document_id: &str,
        title: &str,
        source: &str,
        text: &str,
        strategy: ChunkingStrategy,
        metadata: HashMap<String, String>,
    ) -> Result<Document> {
        let doc = chunk_document(document_id, title, source, text, &strategy, metadata);
        self.rag_documents.write().unwrap()
            .entry(collection.to_string())
            .or_default()
            .push(doc.clone());
        info!("RAG document ingested: {} into collection {}", document_id, collection);
        Ok(doc)
    }

    pub fn rag_list_documents(&self, collection: &str) -> Vec<Document> {
        self.rag_documents.read().unwrap()
            .get(collection).cloned()
            .unwrap_or_default()
    }

    pub fn rag_clear(&self, collection: &str) {
        self.rag_documents.write().unwrap().remove(collection);
    }

    // ── Analytics Operations ──
    pub fn analytics_kmeans(&self, table: &str, k: usize, iterations: usize) -> Result<Vec<ClusterResult>> {
        let vectors = self.load_all_vectors(table)?;
        Ok(kmeans_clustering(&vectors, k, iterations))
    }

    pub fn analytics_anomalies(&self, table: &str, threshold: f32) -> Result<Vec<AnomalyResult>> {
        let vectors = self.load_all_vectors(table)?;
        Ok(detect_anomalies(&vectors, threshold))
    }

    pub fn analytics_vector_profile(&self, table: &str) -> Result<serde_json::Value> {
        let vectors = self.load_all_vectors(table)?;
        if vectors.is_empty() {
            return Ok(serde_json::json!({"error": "no vectors"}));
        }
        let n = vectors.len();
        let dim = vectors[0].1.len();

        // Per-dimension statistics
        let mut per_dim_min = vec![f32::MAX; dim];
        let mut per_dim_max = vec![f32::MIN; dim];
        let mut per_dim_mean = vec![0.0f32; dim];

        for (_, v) in &vectors {
            for d in 0..dim {
                per_dim_min[d] = per_dim_min[d].min(v[d]);
                per_dim_max[d] = per_dim_max[d].max(v[d]);
                per_dim_mean[d] += v[d];
            }
        }
        for d in 0..dim { per_dim_mean[d] /= n as f32; }

        // Compute centroid
        let centroid: Vec<f32> = per_dim_mean.clone();

        // Distance distribution from centroid
        let distances: Vec<f32> = vectors.iter().map(|(_, v)| euclidean_distance(v, &centroid)).collect();
        let mean_dist: f32 = distances.iter().sum::<f32>() / n as f32;
        let std_dist: f32 = (distances.iter().map(|d| (d - mean_dist).powi(2)).sum::<f32>() / n as f32).sqrt();

        Ok(serde_json::json!({
            "table": table,
            "total_vectors": n,
            "dimension": dim,
            "centroid": centroid,
            "mean_distance_from_centroid": mean_dist,
            "std_distance_from_centroid": std_dist,
            "per_dimension_ranges": per_dim_min.iter().zip(per_dim_max.iter()).enumerate().map(|(i, (mn, mx))| {
                serde_json::json!({"dim": i, "min": mn, "max": mx, "range": mx - mn})
            }).collect::<Vec<_>>(),
        }))
    }

    // ── Quantized insert ──
    pub fn insert_quantized(
        &self,
        table: &str,
        id: u64,
        vector: &[f32],
        data: serde_json::Value,
        quant: QuantizationType,
    ) -> Result<()> {
        let table_key = format!("table:{}", table);
        let tree = self.db.open_tree(table_key)?;

        let record = if let Some(mut obj) = data.as_object().cloned() {
            obj.insert("vector".to_string(), serde_json::Value::Array(
                vector.iter().map(|&v| serde_json::Value::Number(serde_json::Number::from_f64(v as f64).unwrap())).collect()
            ));
            match quant {
                QuantizationType::Scalar8 => {
                    // Train on-the-fly (in production, pre-train)
                    let sq = ScalarQuantizer::train(&[vector.to_vec()]);
                    let quantized = sq.quantize(vector);
                    obj.insert("_sq_min".to_string(), serde_json::json!(sq.min));
                    obj.insert("_sq_range".to_string(), serde_json::json!(sq.range));
                    obj.insert("_sq_bytes".to_string(), serde_json::json!(quantized));
                }
                QuantizationType::Binary => {
                    let bq = binary_quantize(vector);
                    obj.insert("_bq_sig".to_string(), serde_json::json!(bq));
                    let median = {
                        let mut sorted = vector.to_vec();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        sorted[sorted.len() / 2]
                    };
                    obj.insert("_bq_median".to_string(), serde_json::json!(median));
                }
                QuantizationType::None => {}
            }
            serde_json::Value::Object(obj)
        } else {
            data
        };

        let key = id.to_be_bytes();
        let value = serde_json::to_vec(&record)?;
        tree.insert(key, value)?;
        tree.flush()?;
        Ok(())
    }
}

/// Fallback brute-force search when no ANN index is available
fn fallback_bruteforce(query: &[f32], vectors: &[(String, Vec<f32>)], top_k: usize) -> Vec<(String, f32)> {
    let mut scored: Vec<(String, f32)> = vectors.iter().map(|(id, v)| {
        let sim = cosine_similarity(query, v);
        (id.clone(), sim)
    }).collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}

// ═══════════════════════════════════════════════════════════════════════════════
// StorageEngine Trait Implementation
// ═══════════════════════════════════════════════════════════════════════════════

#[async_trait]
impl StorageEngine for VectorEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let id = self.id_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table);
            let data = data.clone();
            move || -> crate::Result<u64> {
                let tree = db.open_tree(table_key)?;
                let key = id.to_be_bytes();
                let value = serde_json::to_vec(&data)?;
                tree.insert(key, value)?;
                tree.flush()?;
                Ok(id)
            }
        }).await??;

        // Update metrics
        self.metrics.vector_count.fetch_add(1, Ordering::Relaxed);

        // Update HNSW index if active
        if let Some(vector) = data.get("vector").and_then(|v| v.as_array()) {
            let v: Vec<f32> = vector.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
            if !v.is_empty() {
                let id_str = result.to_string();
                // Try to insert into HNSW index
                if self.hnsw_indices.lock().unwrap().contains_key(table) {
                    if let Some(idx) = self.hnsw_indices.lock().unwrap().get_mut(table) {
                        idx.insert(&id_str, v);
                    }
                }
            }
        }

        Ok(result)
    }

    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let start = Instant::now();
        let table_owned = table.to_string();
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);

        let result: Vec<Record> = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table_owned);
            let conditions = conditions.clone();
            let hnsw_indices = self.hnsw_indices.clone();
            let ivf_indices = self.ivf_indices.clone();
            move || -> crate::Result<Vec<Record>> {
                let tree = db.open_tree(table_key)?;
                let mut records_map: HashMap<String, (u64, serde_json::Value)> = HashMap::new();
                let mut vectors: Vec<(String, Vec<f32>)> = Vec::new();

                for item in tree.iter() {
                    let (key, value) = item?;
                    let id = if key.len() == 8 {
                        u64::from_be_bytes(key.as_ref().try_into().unwrap())
                    } else {
                        continue;
                    };
                    let data: serde_json::Value = serde_json::from_slice(&value)?;
                    let id_str = id.to_string();

                    // Legacy exact-match filtering (for non-query_vector conditions)
                    if !Self::matches_conditions(&data, &conditions) {
                        continue;
                    }

                    if let Some(vec_array) = data.get("vector").and_then(|v| v.as_array()) {
                        let vec: Vec<f32> = vec_array.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect();
                        vectors.push((id_str.clone(), vec));
                    }
                    records_map.insert(id_str, (id, data));
                }

                let query_vector: Option<Vec<f32>> = conditions
                    .get("query_vector")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect());

                let top_k = if limit == 0 { 10 } else { limit as usize };

                let sorted_ids: Vec<String> = if let Some(ref qv) = query_vector {
                    // Determine index method from stored config or conditions
                    let method = conditions.get("index_method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("auto");

                    let results = match method {
                        "hnsw" => {
                            hnsw_indices.lock().unwrap().get(&table_owned).cloned()
                                .map(|idx| idx.search(qv, top_k))
                                .unwrap_or_else(|| fallback_bruteforce(qv, &vectors, top_k))
                        }
                        "ivf" => {
                            ivf_indices.lock().unwrap().get(&table_owned).cloned()
                                .map(|idx| idx.search(qv, top_k))
                                .unwrap_or_else(|| fallback_bruteforce(qv, &vectors, top_k))
                        }
                        _ => {
                            // Try HNSW first, then IVF, then brute-force
                            let hnsw = hnsw_indices.lock().unwrap().get(&table_owned).cloned();
                            if let Some(idx) = hnsw {
                                if !idx.is_empty() {
                                    idx.search(qv, top_k)
                                } else {
                                    let ivf = ivf_indices.lock().unwrap().get(&table_owned).cloned();
                                    ivf.map(|idx| idx.search(qv, top_k))
                                        .unwrap_or_else(|| fallback_bruteforce(qv, &vectors, top_k))
                                }
                            } else {
                                let ivf = ivf_indices.lock().unwrap().get(&table_owned).cloned();
                                ivf.map(|idx| idx.search(qv, top_k))
                                    .unwrap_or_else(|| fallback_bruteforce(qv, &vectors, top_k))
                            }
                        }
                    };

                    results.into_iter().map(|(id, _)| id).collect()
                } else {
                    // Non-vector query: apply filters and paginate
                    let mut all_ids: Vec<u64> = records_map.values().map(|(id, _)| *id).collect();
                    all_ids.sort();
                    all_ids.into_iter().skip(offset as usize).take(top_k).map(|id| id.to_string()).collect()
                };

                let mut records = Vec::new();
                for id_str in sorted_ids {
                    if let Some((_id, data)) = records_map.remove(&id_str) {
                        records.push(Record {
                            id: id_str,
                            data,
                            metadata: HashMap::new(),
                        });
                    }
                }

                Ok(records)
            }
        }).await??;

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.metrics.record_query(elapsed);

        Ok(result)
    }

    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);
        let data = data.clone();
        let table_owned = table.to_string();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table_owned);
            move || -> crate::Result<u64> {
                let tree = db.open_tree(table_key)?;
                let mut updated = 0u64;
                let mut batch = Vec::new();

                for item in tree.iter() {
                    let (key, value) = item?;
                    let stored: serde_json::Value = serde_json::from_slice(&value)?;

                    if Self::matches_conditions(&stored, &conditions) {
                        let merged = if let (Some(stored_obj), Some(data_obj)) =
                            (stored.as_object(), data.as_object())
                        {
                            let mut merged = stored_obj.clone();
                            for (k, v) in data_obj {
                                merged.insert(k.clone(), v.clone());
                            }
                            serde_json::Value::Object(merged)
                        } else {
                            data.clone()
                        };
                        let new_value = serde_json::to_vec(&merged)?;
                        batch.push((key.to_vec(), new_value));
                        updated += 1;
                    }
                }

                for (key, value) in batch {
                    tree.insert(key, value)?;
                }

                if updated > 0 { tree.flush()?; }
                info!("Vector update in {}: {} records updated", table_owned, updated);
                Ok(updated)
            }
        }).await??;

        Ok(result)
    }

    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);
        let table_owned = table.to_string();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table_owned);
            move || -> crate::Result<u64> {
                let tree = db.open_tree(table_key)?;
                let mut deleted = 0u64;
                let mut to_remove = Vec::new();

                for item in tree.iter() {
                    let (key, value) = item?;
                    let stored: serde_json::Value = serde_json::from_slice(&value)?;

                    if Self::matches_conditions(&stored, &conditions) {
                        to_remove.push(key.to_vec());
                        deleted += 1;
                    }
                }

                for key in &to_remove { tree.remove(key)?; }
                if deleted > 0 { tree.flush()?; }
                info!("Vector delete from {}: {} records deleted", table_owned, deleted);
                Ok(deleted)
            }
        }).await??;

        let deleted = result;
        self.metrics.vector_count.fetch_sub(deleted, Ordering::Relaxed);
        self.metrics.deleted_vector_count.fetch_add(deleted, Ordering::Relaxed);

        Ok(result)
    }

    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        let table_owned = table.to_string();
        let metrics_json = self.metrics.to_json();
        let col_config = self.collection_configs.read().unwrap().get(table).cloned();
        let hnsw_size = self.hnsw_indices.lock().unwrap().get(table).map(|i| i.len()).unwrap_or(0);
        let ivf_size = self.ivf_indices.lock().unwrap().get(table).map(|i| i.len()).unwrap_or(0);

        let result: serde_json::Value = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table_owned);
            move || -> crate::Result<serde_json::Value> {
                let tree = db.open_tree(table_key)?;
                let mut total_records = 0u64;
                let mut dim: Option<usize> = None;
                let mut field_counts: HashMap<String, u64> = HashMap::new();
                let mut field_types: HashMap<String, String> = HashMap::new();

                for item in tree.iter() {
                    let (_, value) = item?;
                    total_records += 1;
                    let data: serde_json::Value = serde_json::from_slice(&value)?;

                    if let Some(vec_array) = data.get("vector").and_then(|v| v.as_array()) {
                        if dim.is_none() { dim = Some(vec_array.len()); }
                    }

                    if let Some(obj) = data.as_object() {
                        for (key, val) in obj {
                            if key == "vector" { continue; }
                            *field_counts.entry(key.clone()).or_insert(0) += 1;
                            if !field_types.contains_key(key) {
                                let type_str = match val {
                                    serde_json::Value::Null => "null",
                                    serde_json::Value::Bool(_) => "boolean",
                                    serde_json::Value::Number(_) => "number",
                                    serde_json::Value::String(_) => "string",
                                    serde_json::Value::Array(_) => "array",
                                    serde_json::Value::Object(_) => "object",
                                };
                                field_types.insert(key.clone(), type_str.to_string());
                            }
                        }
                    }
                }

                Ok(serde_json::json!({
                    "table": table_owned,
                    "total_records": total_records,
                    "dimension": dim,
                    "fields": field_counts,
                    "field_types": field_types,
                    "engine": "vector",
                    "index_hnsw_size": hnsw_size,
                    "index_ivf_size": ivf_size,
                    "collection_config": col_config,
                    "metrics": metrics_json,
                }))
            }
        }).await??;

        info!("Vector analyze for table: {} - {} records", table, result["total_records"]);
        Ok(serde_json::to_string(&result)?)
    }

    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table);
            move || -> crate::Result<()> {
                db.open_tree(table_key)?;
                Ok(())
            }
        }).await??;

        info!("Vector collection created: {}", table);
        Ok(())
    }

    async fn drop_table(&self, table: &str) -> Result<()> {
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table);
            move || -> crate::Result<()> {
                db.drop_tree(table_key)?;
                Ok(())
            }
        }).await??;

        // Clean up in-memory state
        self.hnsw_indices.lock().unwrap().remove(table);
        self.ivf_indices.lock().unwrap().remove(table);
        self.collection_configs.write().unwrap().remove(table);
        self.rag_documents.write().unwrap().remove(table);

        info!("Vector collection dropped: {}", table);
        Ok(())
    }

    async fn truncate_table(&self, table: &str, _cascade: bool) -> Result<()> {
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table);
            move || -> crate::Result<()> {
                let tree = db.open_tree(table_key)?;
                let mut iter = tree.iter();
                while let Some(Ok((key, _))) = iter.next() {
                    tree.remove(key)?;
                }
                tree.flush()?;
                Ok(())
            }
        }).await??;

        // Clean up indices
        self.hnsw_indices.lock().unwrap().remove(table);
        self.ivf_indices.lock().unwrap().remove(table);

        info!("Vector collection truncated: {}", table);
        Ok(())
    }

    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let (count, size): (usize, u64) = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = format!("table:{}", table);
            move || -> crate::Result<(usize, u64)> {
                let tree = db.open_tree(table_key)?;
                let count = tree.len();
                let size = tree.iter().filter_map(|item| {
                    item.ok().map(|(_, v)| v.len() as u64)
                }).sum();
                Ok((count, size))
            }
        }).await??;

        info!("Vector collection info retrieved: {} ({} rows)", table, count);
        Ok(TableInfo {
            name: table.to_string(),
            schema: Schema { fields: vec![], indexes: vec![], constraints: vec![] },
            row_count: count as u64,
            size_bytes: size,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);

        let c = vec![2.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &c);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dot = dot_product(&a, &b);
        assert!((dot - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = manhattan_distance(&a, &b);
        assert!((dist - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_matches_conditions_skips_vector_field() {
        let data = serde_json::json!({"name": "test", "vector": [1.0, 2.0]});
        let cond = serde_json::json!({"name": "test"});
        assert!(VectorEngine::matches_conditions(&data, &cond));
    }

    #[test]
    fn test_matches_conditions_with_operator_filter() {
        let data = serde_json::json!({"age": 30, "name": "alice"});
        let cond = serde_json::json!({"age": {"$gte": 25}});
        assert!(VectorEngine::matches_conditions(&data, &cond));

        let cond2 = serde_json::json!({"age": {"$lt": 20}});
        assert!(!VectorEngine::matches_conditions(&data, &cond2));
    }

    #[test]
    fn test_payload_filter_eq() {
        let data = serde_json::json!({"name": "alice", "age": 30});
        let filter = PayloadFilter { field: "name".to_string(), op: FilterOp::Eq(serde_json::json!("alice")) };
        assert!(filter.matches(&data));

        let filter2 = PayloadFilter { field: "name".to_string(), op: FilterOp::Eq(serde_json::json!("bob")) };
        assert!(!filter2.matches(&data));
    }

    #[test]
    fn test_payload_filter_comparison() {
        let data = serde_json::json!({"age": 30, "score": 95.5});

        let filter = PayloadFilter { field: "age".to_string(), op: FilterOp::Gt(serde_json::json!(25)) };
        assert!(filter.matches(&data));

        let filter = PayloadFilter { field: "age".to_string(), op: FilterOp::Lt(serde_json::json!(25)) };
        assert!(!filter.matches(&data));

        let filter = PayloadFilter { field: "score".to_string(), op: FilterOp::Gte(serde_json::json!(90.0)) };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_payload_filter_in_nin() {
        let data = serde_json::json!({"color": "red", "size": "M"});

        let filter = PayloadFilter {
            field: "color".to_string(),
            op: FilterOp::In(vec![serde_json::json!("red"), serde_json::json!("blue")]),
        };
        assert!(filter.matches(&data));

        let filter = PayloadFilter {
            field: "color".to_string(),
            op: FilterOp::Nin(vec![serde_json::json!("green"), serde_json::json!("blue")]),
        };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_payload_filter_exists() {
        let data = serde_json::json!({"name": "alice", "age": 30});

        let filter = PayloadFilter { field: "age".to_string(), op: FilterOp::Exists(true) };
        assert!(filter.matches(&data));

        let filter = PayloadFilter { field: "email".to_string(), op: FilterOp::Exists(true) };
        assert!(!filter.matches(&data));

        let filter = PayloadFilter { field: "email".to_string(), op: FilterOp::Exists(false) };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_payload_filter_regex() {
        let data = serde_json::json!({"email": "alice@example.com"});
        let filter = PayloadFilter {
            field: "email".to_string(),
            op: FilterOp::Regex(r"^[a-z]+@example\.com$".to_string()),
        };
        assert!(filter.matches(&data));

        let data2 = serde_json::json!({"email": "bob@other.com"});
        assert!(!filter.matches(&data2));
    }

    #[test]
    fn test_payload_filter_logical_ops() {
        let data = serde_json::json!({"age": 30, "city": "NYC", "active": true});

        // AND: age > 25 AND city = NYC
        let filter = PayloadFilter {
            field: "".to_string(),
            op: FilterOp::And(vec![
                PayloadFilter { field: "age".to_string(), op: FilterOp::Gt(serde_json::json!(25)) },
                PayloadFilter { field: "city".to_string(), op: FilterOp::Eq(serde_json::json!("NYC")) },
            ]),
        };
        assert!(filter.matches(&data));

        // OR: age > 40 OR city = NYC
        let filter = PayloadFilter {
            field: "".to_string(),
            op: FilterOp::Or(vec![
                PayloadFilter { field: "age".to_string(), op: FilterOp::Gt(serde_json::json!(40)) },
                PayloadFilter { field: "city".to_string(), op: FilterOp::Eq(serde_json::json!("NYC")) },
            ]),
        };
        assert!(filter.matches(&data));

        // NOT: NOT(city = LA)
        let filter = PayloadFilter {
            field: "".to_string(),
            op: FilterOp::Not(Box::new(PayloadFilter {
                field: "city".to_string(), op: FilterOp::Eq(serde_json::json!("LA")),
            })),
        };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_ivf_index_build_and_search_with_preassign() {
        let vectors: Vec<(String, Vec<f32>)> = (0..20).map(|i| {
            (format!("id_{}", i), vec![(i * 3) as f32, (i * 2 + 1) as f32])
        }).collect();
        let index = IvfIndex::build(&vectors, 3, 2, &DistanceMetric::Euclidean);
        let query = vec![57.0, 39.0]; // closest to id_19: [57, 39]
        let results = index.search(&query, 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "id_19");
    }

    #[test]
    fn test_ivf_index_with_exact_dimension_validation() {
        let vectors: Vec<(String, Vec<f32>)> = vec![
            ("a".to_string(), vec![1.0, 2.0, 3.0]),
            ("b".to_string(), vec![4.0, 5.0, 6.0]),
        ];
        let index = IvfIndex::build(&vectors, 2, 2, &DistanceMetric::Cosine);
        let query = vec![1.0, 2.0, 3.0];
        let results = index.search(&query, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_hnsw_index_real_graph_search() {
        let mut index = HnswIndex::new(DistanceMetric::Euclidean, 4, 10, 10);
        let vectors: Vec<(String, Vec<f32>)> = (0..30).map(|i| {
            (format!("id_{}", i), vec![(i * 3) as f32, (i * 2 + 1) as f32])
        }).collect();

        for (id, v) in &vectors {
            index.insert(id, v.clone());
        }

        let query = vec![87.0, 59.0]; // closest to id_29: [87, 59]
        let results = index.search(&query, 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "id_29");
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_hnsw_index_insert_and_search_incremental() {
        let mut index = HnswIndex::new(DistanceMetric::Cosine, 4, 10, 10);

        // Insert first vector
        index.insert("a", vec![1.0, 0.0, 0.0]);
        assert_eq!(index.len(), 1);

        // Insert more vectors
        index.insert("b", vec![0.0, 1.0, 0.0]);
        index.insert("c", vec![1.0, 1.0, 0.0]);
        assert_eq!(index.len(), 3);

        // Search should return results
        let results = index.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
    }

    #[test]
    fn test_hnsw_index_empty() {
        let index = HnswIndex::new(DistanceMetric::Euclidean, 4, 10, 10);
        assert!(index.is_empty());
        let results = index.search(&[1.0, 2.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_scalar_quantization_roundtrip() {
        let vectors = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![5.0, 6.0, 7.0, 8.0],
            vec![9.0, 10.0, 11.0, 12.0],
        ];
        let sq = ScalarQuantizer::train(&vectors);
        let quantized = sq.quantize(&vectors[0]);
        let dequantized = sq.dequantize(&quantized);

        for d in 0..4 {
            let diff = (dequantized[d] - vectors[0][d]).abs();
            assert!(diff < 0.5, "Dimension {}: {} vs {}, diff={}", d, dequantized[d], vectors[0][d], diff);
        }
    }

    #[test]
    fn test_binary_quantization_hamming() {
        let vec_a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let vec_b = vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let sig_a = binary_quantize(&vec_a);
        let sig_b = binary_quantize(&vec_b);

        let hd = hamming_distance(&sig_a, &sig_b);
        // The two vectors are opposites so should have high hamming distance
        assert!(hd > 0);
    }

    #[test]
    fn test_scoring_rrf_fusion() {
        let list1 = vec![
            ScoredResult { id: "a".to_string(), score: 0.9, scores: vec![0.9] },
            ScoredResult { id: "b".to_string(), score: 0.8, scores: vec![0.8] },
            ScoredResult { id: "c".to_string(), score: 0.7, scores: vec![0.7] },
        ];
        let list2 = vec![
            ScoredResult { id: "b".to_string(), score: 0.95, scores: vec![0.95] },
            ScoredResult { id: "c".to_string(), score: 0.85, scores: vec![0.85] },
            ScoredResult { id: "d".to_string(), score: 0.75, scores: vec![0.75] },
        ];

        let fused = reciprocal_rank_fusion(vec![list1, list2], 60, 3);
        assert_eq!(fused.len(), 3);
        // "b" is rank 1 in list2 and rank 2 in list1 → should be first
        assert_eq!(fused[0].id, "b");
    }

    #[test]
    fn test_scoring_weighted_fusion() {
        let list1 = vec![
            ScoredResult { id: "a".to_string(), score: 1.0, scores: vec![1.0] },
            ScoredResult { id: "b".to_string(), score: 0.5, scores: vec![0.5] },
        ];
        let list2 = vec![
            ScoredResult { id: "b".to_string(), score: 1.0, scores: vec![1.0] },
            ScoredResult { id: "a".to_string(), score: 0.0, scores: vec![0.0] },
        ];

        let fused = weighted_fusion(vec![list1, list2], &[0.5, 0.5], 2);
        // a: 0.5*1.0 + 0.5*0.0 = 0.5
        // b: 0.5*0.5 + 0.5*1.0 = 0.75
        assert_eq!(fused[0].id, "b");
    }

    #[test]
    fn test_kmeans_clustering() {
        let vectors: Vec<(String, Vec<f32>)> = (0..20).map(|i| {
            (format!("id_{}", i), vec![(i * 2) as f32, (i * 3) as f32])
        }).collect();
        let clusters = kmeans_clustering(&vectors, 3, 10);
        assert_eq!(clusters.len(), 3);
        assert!(clusters.iter().map(|c| c.size).sum::<usize>() <= 20);
        for cluster in &clusters {
            assert_eq!(cluster.centroid.len(), 2);
        }
    }

    #[test]
    fn test_anomaly_detection() {
        // Create vectors where most are close but one is far
        let mut vectors: Vec<(String, Vec<f32>)> = (0..10).map(|i| {
            (format!("normal_{}", i), vec![1.0 + (i as f32 * 0.1), 2.0 + (i as f32 * 0.1)])
        }).collect();
        vectors.push(("anomaly".to_string(), vec![100.0, 200.0]));

        let results = detect_anomalies(&vectors, 3.0);
        let anomaly = results.iter().find(|r| r.id == "anomaly").unwrap();
        assert!(anomaly.is_anomaly);
    }

    #[test]
    fn test_rag_chunking_fixed() {
        let text = "This is a test document for chunking. It has multiple sentences that should be split into chunks. Each chunk should be of fixed size with optional overlap.";
        let doc = chunk_document(
            "doc1", "Test", "test",
            text,
            &ChunkingStrategy::Fixed { chunk_size: 50, overlap: 10 },
            HashMap::new(),
        );
        assert_eq!(doc.checksum.len(), 64);
        assert!(!doc.chunks.is_empty());
        for chunk in &doc.chunks {
            assert!(chunk.text.len() <= 50);
            assert!(chunk.id.starts_with("doc1_chunk_"));
        }
    }

    #[test]
    fn test_rag_chunking_recursive() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence.";
        let doc = chunk_document(
            "doc2", "Test", "test",
            text,
            &ChunkingStrategy::Recursive {
                chunk_size: 20,
                overlap: 5,
                separators: vec![". ".to_string(), ". ".to_string()],
            },
            HashMap::new(),
        );
        assert!(!doc.chunks.is_empty());
    }

    #[test]
    fn test_rag_chunking_sliding_window() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let doc = chunk_document(
            "doc3", "Test", "test",
            text,
            &ChunkingStrategy::SlidingWindow { chunk_size: 10, overlap: 3 },
            HashMap::new(),
        );
        assert!(!doc.chunks.is_empty());
        for chunk in &doc.chunks {
            assert!(chunk.text.len() <= 10);
        }
    }

    #[test]
    fn test_rag_retrieve_similar_chunks() {
        let mut doc1 = chunk_document(
            "doc1", "Colors", "test",
            "Red is a warm color Blue is a cool color Green is nature",
            &ChunkingStrategy::Fixed { chunk_size: 30, overlap: 5 },
            HashMap::new(),
        );
        // Assign mock embeddings
        for chunk in &mut doc1.chunks {
            chunk.embedding = Some(vec![1.0, 0.0, 0.0]);
        }

        let query = vec![1.0, 0.0, 0.0];
        let results = rag_retrieve_similar_chunks(
            &query, &[doc1], 2, &DistanceMetric::Cosine,
        );
        assert!(!results.is_empty());
        assert!(results[0].1 > 0.99);
    }

    #[test]
    fn test_vector_profile_analytics() {
        let vectors: Vec<(String, Vec<f32>)> = vec![
            ("a".to_string(), vec![1.0, 2.0]),
            ("b".to_string(), vec![3.0, 4.0]),
            ("c".to_string(), vec![5.0, 6.0]),
        ];
        let profile = kmeans_clustering(&vectors, 2, 5);
        assert_eq!(profile.len(), 2);

        let anomalies = detect_anomalies(&vectors, 3.0);
        assert_eq!(anomalies.len(), 3);
    }

    #[test]
    fn test_parse_payload_filters_empty() {
        let conditions = serde_json::json!({});
        let filters = parse_payload_filters(&conditions);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_parse_payload_filters_basic() {
        let conditions = serde_json::json!({"name": "alice", "age": {"$gte": 25}});
        let filters = parse_payload_filters(&conditions);
        // Both exact-match and operator filter are parsed
        assert_eq!(filters.len(), 2);
    }

    #[test]
    fn test_parse_payload_filters_skips_query_vector() {
        let conditions = serde_json::json!({
            "query_vector": [1.0, 2.0],
            "name": {"$eq": "test"}
        });
        let filters = parse_payload_filters(&conditions);
        assert!(!filters.iter().any(|f| f.field == "query_vector"));
        assert!(filters.iter().any(|f| f.field == "name"));
    }

    #[tokio::test]
    async fn test_vector_engine_insert_and_select() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_vec", &schema).await.unwrap();

        let tx = test_tx();
        let data = serde_json::json!({"name": "v1", "vector": [1.0, 2.0, 3.0]});
        let id = engine.insert("test_vec", &data, &tx).await.unwrap();
        assert!(id > 0);

        let records = engine.select("test_vec", None, 10, 0, &tx).await.unwrap();
        assert_eq!(records.len(), 1);

        engine.drop_table("test_vec").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_similarity_search() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_sim", &schema).await.unwrap();

        let tx = test_tx();
        for i in 0..5 {
            let data = serde_json::json!({"id": i, "vector": [i as f32, 5.0 - i as f32]});
            engine.insert("test_sim", &data, &tx).await.unwrap();
        }

        let cond = serde_json::json!({"query_vector": [4.0, 1.0]});
        let records = engine.select("test_sim", Some(&cond), 3, 0, &tx).await.unwrap();
        assert!(!records.is_empty());
        assert_eq!(records[0].data["id"], 4);

        engine.drop_table("test_sim").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_hnsw_index_build_and_search() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_hnsw", &schema).await.unwrap();

        let tx = test_tx();
        for i in 0..10 {
            let data = serde_json::json!({"id": i, "vector": [(i * 3) as f32, (i * 2) as f32]});
            engine.insert("test_hnsw", &data, &tx).await.unwrap();
        }

        // Build HNSW index
        engine.build_hnsw_index("test_hnsw", 4, 10, 10).unwrap();

        // Search with HNSW
        let cond = serde_json::json!({"query_vector": [27.0, 18.0], "index_method": "hnsw"});
        let records = engine.select("test_hnsw", Some(&cond), 3, 0, &tx).await.unwrap();
        assert!(!records.is_empty());

        engine.drop_table("test_hnsw").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_ivf_index_build_and_search() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_ivf", &schema).await.unwrap();

        let tx = test_tx();
        for i in 0..20 {
            let data = serde_json::json!({"id": i, "vector": [(i * 3) as f32, (i * 2 + 1) as f32]});
            engine.insert("test_ivf", &data, &tx).await.unwrap();
        }

        // Build IVF index
        engine.build_ivf_index("test_ivf", 3, 2).unwrap();

        // Search with IVF
        let cond = serde_json::json!({"query_vector": [57.0, 39.0], "index_method": "ivf"});
        let records = engine.select("test_ivf", Some(&cond), 3, 0, &tx).await.unwrap();
        assert!(!records.is_empty());

        engine.drop_table("test_ivf").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_filtered_search() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_filter", &schema).await.unwrap();

        let tx = test_tx();
        let data = serde_json::json!({"name": "alice", "age": 30, "vector": [1.0, 0.0]});
        engine.insert("test_filter", &data, &tx).await.unwrap();
        let data = serde_json::json!({"name": "bob", "age": 25, "vector": [0.0, 1.0]});
        engine.insert("test_filter", &data, &tx).await.unwrap();
        let data = serde_json::json!({"name": "charlie", "age": 35, "vector": [0.5, 0.5]});
        engine.insert("test_filter", &data, &tx).await.unwrap();

        // Filter with operator
        let cond = serde_json::json!({"age": {"$gte": 30}});
        let records = engine.select("test_filter", Some(&cond), 10, 0, &tx).await.unwrap();
        assert_eq!(records.len(), 2);

        // Combined vector + filter
        let cond = serde_json::json!({
            "query_vector": [1.0, 0.0],
            "age": {"$gte": 30}
        });
        let records = engine.select("test_filter", Some(&cond), 10, 0, &tx).await.unwrap();
        assert!(!records.is_empty());

        engine.drop_table("test_filter").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_analytics() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_analytics", &schema).await.unwrap();

        let tx = test_tx();
        for i in 0..10 {
            let data = serde_json::json!({"id": i, "vector": [(i * 2) as f32, (i * 3) as f32]});
            engine.insert("test_analytics", &data, &tx).await.unwrap();
        }

        let clusters = engine.analytics_kmeans("test_analytics", 2, 5).unwrap();
        assert_eq!(clusters.len(), 2);

        let profile = engine.analytics_vector_profile("test_analytics").unwrap();
        assert_eq!(profile["total_vectors"], 10);
        assert!(profile["dimension"].as_u64().unwrap() >= 2);

        engine.drop_table("test_analytics").await.unwrap();
    }

    #[test]
    fn test_hnsw_many_vectors() {
        let mut index = HnswIndex::new(DistanceMetric::Euclidean, 8, 20, 20);
        let n = 50;
        for i in 0..n {
            let v = vec![(i * 2) as f32, (i * 3 + 1) as f32, (i % 10) as f32];
            index.insert(&format!("v{}", i), v);
        }
        assert_eq!(index.len(), n);

        let query = vec![98.0, 148.0, 9.0]; // close to v49: [98, 148, 9]
        let results = index.search(&query, 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "v49");
    }

    #[test]
    fn test_ivf_large_collection() {
        let vectors: Vec<(String, Vec<f32>)> = (0..50).map(|i| {
            (format!("id_{}", i), vec![(i as f32).sin(), (i as f32).cos()])
        }).collect();
        let index = IvfIndex::build(&vectors, 5, 2, &DistanceMetric::Cosine);
        let query = vec![0.0, 1.0];
        let results = index.search(&query, 5);
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_hnsw_insert_duplicate() {
        let mut index = HnswIndex::new(DistanceMetric::Euclidean, 4, 10, 10);
        index.insert("a", vec![1.0, 2.0]);
        index.insert("a", vec![1.0, 2.0]); // duplicate, should be ignored
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_cosine_similarity_normalized() {
        let a = vec![3.0, 4.0];  // magnitude 5
        let b = vec![6.0, 8.0];  // magnitude 10
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_distance_identity() {
        let a = vec![1.0, 2.0, 3.0];
        let dist = euclidean_distance(&a, &a);
        assert!((dist - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_identity() {
        let a = vec![1.0, 2.0, 3.0];
        let dot = dot_product(&a, &a);
        assert!((dot - 14.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_vector_engine_update() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_upd", &schema).await.unwrap();

        let tx = test_tx();
        let data = serde_json::json!({"name": "v1", "value": 10, "vector": [1.0, 2.0]});
        engine.insert("test_upd", &data, &tx).await.unwrap();

        let update = serde_json::json!({"value": 20});
        let updated = engine.update("test_upd", Some(&serde_json::json!({"name": "v1"})), &update, &tx).await.unwrap();
        assert_eq!(updated, 1);

        let records = engine.select("test_upd", None, 10, 0, &tx).await.unwrap();
        assert_eq!(records[0].data["value"], 20);

        engine.drop_table("test_upd").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_delete() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_del", &schema).await.unwrap();

        let tx = test_tx();
        let data = serde_json::json!({"name": "v1", "vector": [1.0, 2.0]});
        engine.insert("test_del", &data, &tx).await.unwrap();

        let deleted = engine.delete("test_del", Some(&serde_json::json!({"name": "v1"})), &tx).await.unwrap();
        assert_eq!(deleted, 1);

        let records = engine.select("test_del", None, 10, 0, &tx).await.unwrap();
        assert_eq!(records.len(), 0);

        engine.drop_table("test_del").await.unwrap();
    }

    #[tokio::test]
    async fn test_vector_engine_table_info() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_info", &schema).await.unwrap();

        let info = engine.table_info("test_info").await.unwrap();
        assert_eq!(info.name, "test_info");
        assert_eq!(info.row_count, 0);

        engine.drop_table("test_info").await.unwrap();
    }

    #[test]
    fn test_sq8_single_vector() {
        let vectors = vec![vec![10.0, 20.0, 30.0, 40.0, 50.0]];
        let sq = ScalarQuantizer::train(&vectors);
        let q = sq.quantize(&vectors[0]);
        assert_eq!(q.len(), 5);
        let dq = sq.dequantize(&q);
        for d in 0..5 {
            let err = (dq[d] - vectors[0][d]).abs();
            assert!(err < (vectors[0][d] * 0.01) + 0.5,
                "SQ8 error too high at dim {}: {} vs {} (err={})", d, dq[d], vectors[0][d], err);
        }
    }

    #[test]
    fn test_rrf_empty_lists() {
        let fused = reciprocal_rank_fusion(vec![], 60, 10);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_weighted_fusion_empty_lists() {
        let fused = weighted_fusion(vec![], &[], 10);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_anomaly_detection_no_anomalies() {
        let vectors: Vec<(String, Vec<f32>)> = (0..5).map(|i| {
            (format!("n{}", i), vec![10.0 + (i as f32 * 0.01), 20.0 + (i as f32 * 0.01)])
        }).collect();
        let results = detect_anomalies(&vectors, 5.0);
        assert!(!results.iter().any(|r| r.is_anomaly));
    }

    #[test]
    fn test_kmeans_single_cluster() {
        let vectors: Vec<(String, Vec<f32>)> = (0..5).map(|i| {
            (format!("id_{}", i), vec![1.0, 2.0, 3.0])
        }).collect();
        let clusters = kmeans_clustering(&vectors, 1, 5);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].size, 5);
    }

    #[test]
    fn test_rag_metadata_preserved() {
        let mut meta = HashMap::new();
        meta.insert("author".to_string(), "test".to_string());
        let doc = chunk_document(
            "doc1", "Test", "test",
            "Hello world chunking test",
            &ChunkingStrategy::Fixed { chunk_size: 10, overlap: 2 },
            meta.clone(),
        );
        assert_eq!(doc.metadata.get("author").unwrap(), "test");
        for chunk in &doc.chunks {
            assert_eq!(chunk.metadata.get("author").unwrap(), "test");
        }
    }

    #[test]
    fn test_hnsw_different_metrics() {
        let mut idx_cos = HnswIndex::new(DistanceMetric::Cosine, 4, 10, 10);
        idx_cos.insert("a", vec![1.0, 0.0]);
        idx_cos.insert("b", vec![0.0, 1.0]);
        let res_cos = idx_cos.search(&[1.0, 0.0], 2);
        assert_eq!(res_cos[0].0, "a");

        let mut idx_l2 = HnswIndex::new(DistanceMetric::Euclidean, 4, 10, 10);
        idx_l2.insert("a", vec![0.0, 0.0]);
        idx_l2.insert("b", vec![3.0, 4.0]);
        let res_l2 = idx_l2.search(&[0.0, 0.0], 2);
        assert_eq!(res_l2[0].0, "a");
    }

    #[test]
    fn test_ivf_empty_input() {
        let index = IvfIndex::build(&[], 5, 2, &DistanceMetric::Cosine);
        assert!(index.centroids.is_empty());
        assert!(index.search(&[1.0, 2.0], 5).is_empty());
    }

    #[test]
    fn test_ivf_brute_force_fallback() {
        let index = IvfIndex::build(&[], 0, 0, &DistanceMetric::Cosine);
        let vectors = vec![("a".to_string(), vec![1.0, 0.0])];
        let results = index.brute_force_search(&[1.0, 0.0], &vectors, 5);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_distance_function_consistency() {
        let a = vec![3.0, 4.0];
        let b = vec![6.0, 8.0];

        let cos = compute_similarity(&a, &b, &DistanceMetric::Cosine);
        let l2 = compute_similarity(&a, &b, &DistanceMetric::Euclidean);
        let dot = compute_similarity(&a, &b, &DistanceMetric::DotProduct);
        let man = compute_similarity(&a, &b, &DistanceMetric::Manhattan);

        assert!(cos > 0.99);  // collinear vectors
        assert!(l2 < 0.0);    // negative because similarity = -distance
        assert!(dot > 0.0);
        assert!(man < 0.0);
    }

    #[test]
    fn test_matches_conditions_with_in_operator() {
        let data = serde_json::json!({"color": "red", "size": "L"});
        let cond = serde_json::json!({"color": {"$in": ["red", "blue"]}});
        assert!(VectorEngine::matches_conditions(&data, &cond));

        let cond = serde_json::json!({"color": {"$in": ["green", "blue"]}});
        assert!(!VectorEngine::matches_conditions(&data, &cond));
    }

    #[test]
    fn test_matches_conditions_with_exists() {
        let data = serde_json::json!({"name": "alice"});
        let cond = serde_json::json!({"name": {"$exists": true}});
        assert!(VectorEngine::matches_conditions(&data, &cond));

        let cond = serde_json::json!({"email": {"$exists": true}});
        assert!(!VectorEngine::matches_conditions(&data, &cond));
    }

    #[test]
    fn test_hnsw_multi_level() {
        let mut index = HnswIndex::new(DistanceMetric::Euclidean, 4, 20, 20);
        // Insert vectors on a unit circle
        for i in 0..20 {
            let theta = (i as f32) * std::f32::consts::PI * 2.0 / 20.0;
            index.insert(&format!("v{}", i), vec![theta.cos(), theta.sin()]);
        }
        assert_eq!(index.len(), 20);

        let results = index.search(&[1.0, 0.0], 3);
        assert!(!results.is_empty());
        // The closest to [1,0] should be v0 (same vector) or nearby
        // Euclidean similarity = -distance, so max is 0 for identical
        assert!(results[0].1 <= 0.0);
        // v0 has the same vector as query, so it should be first
        let first_ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).take(2).collect();
        assert!(first_ids.contains(&"v0") || first_ids.contains(&"v1") || first_ids.contains(&"v19"));
    }

    #[test]
    fn test_scalar_quantization_all_same() {
        let vectors = vec![
            vec![5.0, 5.0, 5.0],
            vec![5.0, 5.0, 5.0],
        ];
        let sq = ScalarQuantizer::train(&vectors);
        let q = sq.quantize(&vectors[0]);
        let dq = sq.dequantize(&q);
        for d in 0..3 {
            assert!((dq[d] - 5.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_payload_filter_and() {
        let data = serde_json::json!({"a": 1, "b": 2, "c": 3});
        let filter = PayloadFilter {
            field: "".to_string(),
            op: FilterOp::And(vec![
                PayloadFilter { field: "a".to_string(), op: FilterOp::Eq(serde_json::json!(1)) },
                PayloadFilter { field: "b".to_string(), op: FilterOp::Eq(serde_json::json!(2)) },
            ]),
        };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_payload_filter_or() {
        let data = serde_json::json!({"a": 1});
        let filter = PayloadFilter {
            field: "".to_string(),
            op: FilterOp::Or(vec![
                PayloadFilter { field: "a".to_string(), op: FilterOp::Eq(serde_json::json!(1)) },
                PayloadFilter { field: "b".to_string(), op: FilterOp::Eq(serde_json::json!(2)) },
            ]),
        };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_payload_filter_not() {
        let data = serde_json::json!({"a": 1});
        let filter = PayloadFilter {
            field: "".to_string(),
            op: FilterOp::Not(Box::new(PayloadFilter {
                field: "a".to_string(), op: FilterOp::Eq(serde_json::json!(2)),
            })),
        };
        assert!(filter.matches(&data));
    }

    #[test]
    fn test_binary_quantization_self_similarity() {
        let vec = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let sig = binary_quantize(&vec);
        let hd = hamming_distance(&sig, &sig);
        assert_eq!(hd, 0);
    }

    #[test]
    fn test_rag_chunking_fixed_no_overlap() {
        let text = "AABBCCDD";
        let doc = chunk_document(
            "doc1", "Test", "test",
            text,
            &ChunkingStrategy::Fixed { chunk_size: 2, overlap: 0 },
            HashMap::new(),
        );
        assert_eq!(doc.chunks.len(), 4);
        assert_eq!(doc.chunks[0].text, "AA");
        assert_eq!(doc.chunks[1].text, "BB");
    }

    #[test]
    fn test_vector_engine_metrics() {
        let metrics = VectorMetrics::new();
        metrics.record_query(10.5);
        metrics.record_query(20.3);
        assert_eq!(metrics.query_count.load(Ordering::Relaxed), 2);
        let avg = metrics.avg_query_latency_ms();
        assert!((avg - 15.4).abs() < 0.01);

        let json = metrics.to_json();
        assert_eq!(json["query_count"], 2);
    }

    #[test]
    fn test_collection_config_default() {
        let config = CollectionConfig::default();
        assert_eq!(config.dimension, 0);
        assert_eq!(config.quantization, QuantizationType::None);
    }

    #[tokio::test]
    async fn test_vector_engine_rag_operations() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = VectorEngine::new(&config).unwrap();

        let doc = engine.rag_ingest_document(
            "rag_col",
            "doc1",
            "Test Doc",
            "manual",
            "This is a test document for RAG operations in PrimusDB vector engine.",
            ChunkingStrategy::Fixed { chunk_size: 30, overlap: 5 },
            HashMap::new(),
        ).unwrap();
        assert_eq!(doc.id, "doc1");
        assert!(!doc.chunks.is_empty());

        let docs = engine.rag_list_documents("rag_col");
        assert_eq!(docs.len(), 1);

        engine.rag_clear("rag_col");
        let docs = engine.rag_list_documents("rag_col");
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn test_sha256_checksum() {
        let hash1 = sha256_checksum("hello");
        let hash2 = sha256_checksum("hello");
        let hash3 = sha256_checksum("world");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test Helpers
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
use crate::transaction::Transaction;

#[cfg(test)]
fn test_tx() -> Transaction {
    Transaction {
        id: "test".to_string(),
        operations: vec![],
        created_at: chrono::Utc::now(),
        status: crate::transaction::TransactionStatus::Active,
        updated_at: chrono::Utc::now(),
        isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
        timeout_ms: 30000,
        ..Default::default()
    }
}
