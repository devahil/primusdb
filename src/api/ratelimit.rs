use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response, Json};
use tokio::sync::RwLock;

/// Configuration for the token-bucket rate limiter (max requests per time window).
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_secs: 60,
        }
    }
}

/// A single token-bucket instance tracking one client's allowance.
///
/// Tokens accrue continuously at `max_tokens / window_secs` per second up to
/// `max_tokens`, and one token is consumed per allowed request.
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    max_tokens: f64,
    refill_rate: f64,
}

impl TokenBucket {
    /// Create a full bucket with the given capacity and refill window.
    fn new(max_tokens: f64, window_secs: u64) -> Self {
        let refill_rate = max_tokens / window_secs as f64;
        Self {
            tokens: max_tokens,
            last_refill: Instant::now(),
            max_tokens,
            refill_rate,
        }
    }

    /// Try to consume one token, refilling the bucket first.
    ///
    /// Returns `true` when at least one token remained, `false` when the
    /// bucket was empty.
    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Add tokens proportional to the time elapsed since the last refill,
    /// capping the balance at `max_tokens`.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

/// Token-bucket rate limiter keyed by client IP or API key.
pub struct RateLimiter {
    buckets: RwLock<HashMap<String, TokenBucket>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a rate limiter from the given token-bucket configuration.
    ///
    /// # Arguments
    /// * `config` - Bucket capacity and refill window; see [`RateLimitConfig`].
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Try to consume one token for `key`, refilling the bucket first.
    ///
    /// # Returns
    /// A tuple of `(allowed, remaining, retry_after_secs)`:
    /// * `allowed` - `true` if the request is permitted, `false` otherwise.
    /// * `remaining` - Tokens left in the bucket after this call (floor).
    /// * `retry_after_secs` - Seconds to wait before retrying; `0` when the
    ///   request was allowed.
    pub async fn check(&self, key: &str) -> (bool, u32, u64) {
        let mut buckets = self.buckets.write().await;
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(self.config.max_requests as f64, self.config.window_secs)
        });

        let allowed = bucket.try_acquire();
        let remaining = bucket.tokens as u32;
        let retry_after = if allowed { 0 } else { self.config.window_secs };

        (allowed, remaining, retry_after)
    }

    /// Remove buckets that have not been touched for two full windows,
    /// bounding the memory footprint of the limiter.
    pub async fn cleanup_old_entries(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let max_age = self.config.window_secs * 2;
        buckets.retain(|_, bucket| now.duration_since(bucket.last_refill).as_secs() < max_age);
    }

    /// Return the active [`RateLimitConfig`].
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}

/// Global Axum middleware enforcing the per-client token-bucket rate limit.
///
/// The client is keyed by the `X-API-Key` header when present, otherwise by
/// client IP (from `X-Forwarded-For` or the connection info). Exceeded limits
/// produce a `429 TOO_MANY_REQUESTS` JSON body with a `retry_after_secs` hint.
/// Otherwise the request proceeds and the response gains `X-RateLimit-Limit`
/// and `X-RateLimit-Remaining` headers as the middleware stack unwinds.
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let client_key = extract_client_key(&request);

    let (allowed, remaining, retry_after) = limiter.check(&client_key).await;

    if !allowed {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": "rate_limit_exceeded",
                "message": "Too many requests",
                "retry_after_secs": retry_after
            })),
        ));
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    if let Ok(val) = limiter.config.max_requests.to_string().parse() {
        headers.insert("X-RateLimit-Limit", val);
    }
    if let Ok(val) = remaining.to_string().parse() {
        headers.insert("X-RateLimit-Remaining", val);
    }

    Ok(response)
}

/// Derive the rate-limit key for a request.
///
/// Returns `api:<key>` when an `X-API-Key` header is present, otherwise
/// `ip:<address>` taken from the first `X-Forwarded-For` entry or the socket
/// peer, defaulting to `127.0.0.1`.
fn extract_client_key(req: &Request) -> String {
    if let Some(api_key) = req.headers().get("X-API-Key") {
        if let Ok(key) = api_key.to_str() {
            return format!("api:{}", key);
        }
    }
    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse::<IpAddr>().ok())
        .or_else(|| {
            req.extensions()
                .get::<axum::extract::ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    format!("ip:{}", ip)
}
