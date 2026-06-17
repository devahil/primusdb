use crate::cluster::domain::DataDomainManager;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum NodeSelectorStrategy {
    RoundRobin,
    #[default]
    LeastLoaded,
    LowestLatency,
    ShardAware,
    Random,
    DomainAware,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayNode {
    pub node_id: String,
    pub host: String,
    pub port: u16,
    pub health: NodeHealth,
    pub active_connections: u32,
    pub ewma_latency_ms: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub last_seen: u64,
    pub consecutive_failures: u32,
    pub circuit_open: bool,
    pub circuit_open_until: u64,
    pub shards: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub strategy: NodeSelectorStrategy,
    pub health_check_interval_ms: u64,
    pub health_check_timeout_ms: u64,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_reset_ms: u64,
    pub ewma_alpha: f64,
    pub max_connections_per_node: u32,
    pub connection_timeout_ms: u64,
    pub dns_cache_ttl_ms: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            strategy: NodeSelectorStrategy::LeastLoaded,
            health_check_interval_ms: 5000,
            health_check_timeout_ms: 2000,
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_ms: 30000,
            ewma_alpha: 0.3,
            max_connections_per_node: 1000,
            connection_timeout_ms: 5000,
            dns_cache_ttl_ms: 60000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub node_id: String,
    pub host: String,
    pub port: u16,
    pub strategy_used: NodeSelectorStrategy,
    pub estimated_latency_ms: f64,
    pub node_health: NodeHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub routed_requests: u64,
    pub failed_requests: u64,
    pub circuit_breaks_triggered: u64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub active_nodes: usize,
    pub healthy_nodes: usize,
    pub strategy: NodeSelectorStrategy,
}

pub struct ClusterGateway {
    pub node_id: String,
    pub config: GatewayConfig,
    pub nodes: RwLock<Vec<GatewayNode>>,
    pub round_robin_index: RwLock<usize>,
    pub metrics: RwLock<GatewayMetrics>,
    pub latencies: RwLock<Vec<u64>>,
    pub http_client: reqwest::Client,
    pub domain_manager: Option<Arc<DataDomainManager>>,
    pub cross_cluster_clients: RwLock<HashMap<String, reqwest::Client>>,
}

impl ClusterGateway {
    pub fn new(node_id: String, config: GatewayConfig) -> Self {
        let strategy = config.strategy;
        let connection_timeout = config.connection_timeout_ms;
        let max_connections = config.max_connections_per_node;
        Self {
            node_id,
            config,
            nodes: RwLock::new(Vec::new()),
            round_robin_index: RwLock::new(0),
            metrics: RwLock::new(GatewayMetrics {
                total_requests: 0,
                routed_requests: 0,
                failed_requests: 0,
                circuit_breaks_triggered: 0,
                avg_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                active_nodes: 0,
                healthy_nodes: 0,
                strategy,
            }),
            latencies: RwLock::new(Vec::with_capacity(1000)),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_millis(connection_timeout))
                .pool_max_idle_per_host(max_connections as usize)
                .build()
                .unwrap_or_default(),
            domain_manager: None,
            cross_cluster_clients: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_domain_manager(mut self, dm: Arc<DataDomainManager>) -> Self {
        self.domain_manager = Some(dm);
        self
    }

    pub async fn register_node(&self, node_id: &str, host: &str, port: u16, shards: Vec<String>) {
        let mut nodes = self.nodes.write().await;
        if let Some(existing) = nodes.iter_mut().find(|n| n.node_id == node_id) {
            existing.host = host.to_string();
            existing.port = port;
            existing.shards = shards;
            existing.health = NodeHealth::Healthy;
            existing.last_seen = now_ms();
        } else {
            nodes.push(GatewayNode {
                node_id: node_id.to_string(),
                host: host.to_string(),
                port,
                health: NodeHealth::Unknown,
                active_connections: 0,
                ewma_latency_ms: 50.0,
                cpu_usage: 0.0,
                memory_usage: 0.0,
                last_seen: now_ms(),
                consecutive_failures: 0,
                circuit_open: false,
                circuit_open_until: 0,
                shards,
            });
        }
    }

    pub async fn remove_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        nodes.retain(|n| n.node_id != node_id);
    }

    pub async fn record_success(&self, node_id: &str, latency_ms: f64) {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.consecutive_failures = 0;
            node.ewma_latency_ms = self.config.ewma_alpha * latency_ms
                + (1.0 - self.config.ewma_alpha) * node.ewma_latency_ms;
            node.health = NodeHealth::Healthy;
            node.last_seen = now_ms();
        }

        let mut latencies = self.latencies.write().await;
        latencies.push(latency_ms as u64);
        if latencies.len() > 10000 {
            latencies.remove(0);
        }

        let mut metrics = self.metrics.write().await;
        if !latencies.is_empty() {
            let sum: u64 = latencies.iter().sum();
            metrics.avg_latency_ms = sum as f64 / latencies.len() as f64;

            let mut sorted = latencies.clone();
            sorted.sort_unstable();
            let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
            metrics.p99_latency_ms =
                *sorted.get(p99_idx.min(sorted.len() - 1)).unwrap_or(&0) as f64;
        }
    }

    pub async fn record_failure(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.iter_mut().find(|n| n.node_id == node_id) {
            node.consecutive_failures += 1;
            if node.consecutive_failures >= self.config.circuit_breaker_threshold {
                node.circuit_open = true;
                node.circuit_open_until = now_ms() + self.config.circuit_breaker_reset_ms;
                node.health = NodeHealth::Unhealthy;

                let mut metrics = self.metrics.write().await;
                metrics.circuit_breaks_triggered += 1;
                warn!(
                    "Circuit breaker opened for node {} ({} consecutive failures)",
                    node_id, node.consecutive_failures
                );
            } else {
                node.health = NodeHealth::Degraded;
            }
        }

        let mut metrics = self.metrics.write().await;
        metrics.failed_requests += 1;
    }

    // === Federation-aware routing ===

    pub async fn get_domain_route(
        &self,
        domain_name: &str,
        _storage_type: &str,
        collection: &str,
    ) -> Result<RouteDecision> {
        if let Some(ref dm) = self.domain_manager {
            if let Some(domain) = dm.get_domain(domain_name).await {
                let members = &domain.member_clusters;
                if members.len() > 1 {
                    let self_idx = members.iter().position(|c| c == &self.node_id);

                    // Route to local cluster if it's a member
                    if self_idx.is_some() {
                        let nodes = self.nodes.read().await;
                        let local = nodes.iter().find(|n| n.node_id == self.node_id);
                        if let Some(n) = local {
                            return Ok(RouteDecision {
                                node_id: n.node_id.clone(),
                                host: n.host.clone(),
                                port: n.port,
                                strategy_used: NodeSelectorStrategy::DomainAware,
                                estimated_latency_ms: n.ewma_latency_ms,
                                node_health: n.health,
                            });
                        }
                    }

                    // Route to first remote cluster member
                    for member in members {
                        if Some(member) != self_idx.map(|i| &members[i]) {
                            let nodes = self.nodes.read().await;
                            if let Some(n) = nodes.iter().find(|n| n.node_id == *member) {
                                return Ok(RouteDecision {
                                    node_id: n.node_id.clone(),
                                    host: n.host.clone(),
                                    port: n.port,
                                    strategy_used: NodeSelectorStrategy::DomainAware,
                                    estimated_latency_ms: n.ewma_latency_ms,
                                    node_health: n.health,
                                });
                            }
                        }
                    }
                }
            }

            // Fallback to cross-cluster via federation
            if let Some(dm) = self.domain_manager.as_ref() {
                if let Some(domain) = dm.get_domain(domain_name).await {
                    for member in &domain.member_clusters {
                        if member != &self.node_id {
                            let nodes = self.nodes.read().await;
                            if let Some(n) = nodes.iter().find(|n| n.node_id == *member) {
                                return Ok(RouteDecision {
                                    node_id: n.node_id.clone(),
                                    host: n.host.clone(),
                                    port: n.port,
                                    strategy_used: NodeSelectorStrategy::DomainAware,
                                    estimated_latency_ms: n.ewma_latency_ms,
                                    node_health: n.health,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Fallback to regular routing
        self.get_route(Some(collection), None).await
    }

    pub async fn get_route(
        &self,
        shard_key: Option<&str>,
        preferred_nodes: Option<&[String]>,
    ) -> Result<RouteDecision> {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;

        // Domain-aware strategy check
        if let NodeSelectorStrategy::DomainAware = self.config.strategy {
            if let (Some(key), Some(dm)) = (shard_key, self.domain_manager.as_ref()) {
                let domains = dm.list_domains().await;
                for domain in &domains {
                    if domain.collections.iter().any(|c| key.starts_with(c))
                        || domain.tables.iter().any(|t| key.starts_with(t))
                    {
                        let local = self.nodes.read().await;
                        if let Some(n) = local.iter().find(|n| n.node_id == self.node_id) {
                            metrics.routed_requests += 1;
                            return Ok(RouteDecision {
                                node_id: n.node_id.clone(),
                                host: n.host.clone(),
                                port: n.port,
                                strategy_used: NodeSelectorStrategy::DomainAware,
                                estimated_latency_ms: n.ewma_latency_ms,
                                node_health: n.health,
                            });
                        }
                    }
                }
            }
        }

        let nodes = self.nodes.read().await;
        let healthy: Vec<&GatewayNode> = nodes
            .iter()
            .filter(|n| n.health == NodeHealth::Healthy || n.health == NodeHealth::Degraded)
            .filter(|n| !n.circuit_open || n.circuit_open_until <= now_ms())
            .collect();

        if healthy.is_empty() {
            return Err(crate::Error::ClusterError(
                "No healthy nodes available".into(),
            ));
        }

        // If preferred nodes specified, try them first
        if let Some(pref) = preferred_nodes {
            for node_id in pref {
                if let Some(node) = healthy.iter().find(|n| n.node_id == *node_id) {
                    metrics.routed_requests += 1;
                    return Ok(RouteDecision {
                        node_id: node.node_id.clone(),
                        host: node.host.clone(),
                        port: node.port,
                        strategy_used: NodeSelectorStrategy::ShardAware,
                        estimated_latency_ms: node.ewma_latency_ms,
                        node_health: node.health,
                    });
                }
            }
        }

        // Shard-aware: find nodes hosting the target shard
        if let Some(key) = shard_key {
            let shard_nodes: Vec<&GatewayNode> = healthy
                .iter()
                .copied()
                .filter(|n| n.shards.iter().any(|s| key.starts_with(s)))
                .collect();
            if !shard_nodes.is_empty() {
                let node = select_best_node(
                    &shard_nodes,
                    &self.config,
                    &self.round_robin_index,
                    &self.nodes,
                )
                .await;
                metrics.routed_requests += 1;
                return Ok(node);
            }
        }

        // Strategy-based selection from all healthy nodes
        let node =
            select_best_node(&healthy, &self.config, &self.round_robin_index, &self.nodes).await;
        metrics.routed_requests += 1;
        Ok(node)
    }

    pub async fn run_health_checks(&self) {
        loop {
            tokio::time::sleep(Duration::from_millis(self.config.health_check_interval_ms)).await;

            let nodes = self.nodes.read().await;
            let node_list: Vec<(String, String, u16)> = nodes
                .iter()
                .map(|n| (n.node_id.clone(), n.host.clone(), n.port))
                .collect();
            drop(nodes);

            for (node_id, host, port) in &node_list {
                let url = format!("http://{}:{}/health", host, port);
                let start = Instant::now();
                match tokio::time::timeout(
                    Duration::from_millis(self.config.health_check_timeout_ms),
                    self.http_client.get(&url).send(),
                )
                .await
                {
                    Ok(Ok(resp)) if resp.status().is_success() => {
                        let latency = start.elapsed().as_secs_f64() * 1000.0;
                        self.record_success(node_id, latency).await;
                    }
                    _ => {
                        self.record_failure(node_id).await;
                    }
                }
            }

            // Update metrics
            {
                let nodes = self.nodes.read().await;
                let mut metrics = self.metrics.write().await;
                metrics.active_nodes = nodes.len();
                metrics.healthy_nodes = nodes
                    .iter()
                    .filter(|n| n.health == NodeHealth::Healthy)
                    .count();
            }
        }
    }

    pub async fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn get_nodes(&self) -> Vec<GatewayNode> {
        self.nodes.read().await.clone()
    }

    pub async fn get_healthy_nodes(&self) -> Vec<GatewayNode> {
        self.nodes
            .read()
            .await
            .iter()
            .filter(|n| n.health == NodeHealth::Healthy && !n.circuit_open)
            .cloned()
            .collect()
    }

    pub async fn send_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
        shard_key: Option<&str>,
    ) -> Result<serde_json::Value> {
        let route = self.get_route(shard_key, None).await?;
        let url = format!("http://{}:{}{}", route.host, route.port, path);
        let start = Instant::now();

        let result = match method {
            reqwest::Method::GET => self.http_client.get(&url).send().await,
            reqwest::Method::POST => {
                let mut req = self.http_client.post(&url);
                if let Some(b) = body {
                    req = req.json(&b);
                }
                req.send().await
            }
            reqwest::Method::PUT => {
                let mut req = self.http_client.put(&url);
                if let Some(b) = body {
                    req = req.json(&b);
                }
                req.send().await
            }
            reqwest::Method::DELETE => self.http_client.delete(&url).send().await,
            _ => {
                return Err(crate::Error::ClusterError("Unsupported HTTP method".into()));
            }
        };

        match result {
            Ok(resp) => {
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                if resp.status().is_success() {
                    self.record_success(&route.node_id, latency).await;
                    let json: serde_json::Value =
                        resp.json().await.unwrap_or(serde_json::Value::Null);
                    Ok(json)
                } else {
                    let status = resp.status().as_u16();
                    self.record_failure(&route.node_id).await;
                    Err(crate::Error::ClusterError(format!(
                        "Request to {} failed with status {}",
                        route.node_id, status
                    )))
                }
            }
            Err(e) => {
                self.record_failure(&route.node_id).await;
                Err(crate::Error::ClusterError(format!(
                    "Request to {} failed: {}",
                    route.node_id, e
                )))
            }
        }
    }
}

async fn select_best_node(
    candidates: &[&GatewayNode],
    config: &GatewayConfig,
    round_robin_index: &RwLock<usize>,
    _nodes: &RwLock<Vec<GatewayNode>>,
) -> RouteDecision {
    if candidates.is_empty() {
        return RouteDecision {
            node_id: "none".into(),
            host: "0.0.0.0".into(),
            port: 0,
            strategy_used: config.strategy,
            estimated_latency_ms: 0.0,
            node_health: NodeHealth::Unknown,
        };
    }

    let node = match config.strategy {
        NodeSelectorStrategy::RoundRobin => {
            let mut idx = round_robin_index.write().await;
            let selected = &candidates[*idx % candidates.len()];
            *idx += 1;
            (*selected).clone()
        }
        NodeSelectorStrategy::LeastLoaded => candidates
            .iter()
            .min_by_key(|n| n.active_connections)
            .map(|n| (*n).clone())
            .unwrap_or_else(|| candidates[0].clone()),
        NodeSelectorStrategy::LowestLatency => candidates
            .iter()
            .min_by(|a, b| a.ewma_latency_ms.partial_cmp(&b.ewma_latency_ms).unwrap())
            .map(|n| (*n).clone())
            .unwrap_or_else(|| candidates[0].clone()),
        NodeSelectorStrategy::ShardAware
        | NodeSelectorStrategy::Random
        | NodeSelectorStrategy::DomainAware => {
            let idx = fast_random_usize() % candidates.len();
            candidates[idx].clone()
        }
    };

    RouteDecision {
        node_id: node.node_id.clone(),
        host: node.host.clone(),
        port: node.port,
        strategy_used: config.strategy,
        estimated_latency_ms: node.ewma_latency_ms,
        node_health: node.health,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn fast_random_usize() -> usize {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    Instant::now().hash(&mut hasher);
    hasher.finish() as usize
}
