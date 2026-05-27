//! Simple token-bucket rate limiter middleware
//!
//! Per-IP rate limiting for admin API endpoints.
//! Uses a simple in-memory sliding window counter.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use tokio::sync::Mutex;

/// Per-client rate limiting state
#[derive(Debug)]
struct ClientBucket {
    /// Timestamp of the start of the current window
    window_start: Instant,
    /// Number of requests in the current window
    count: u32,
}

/// Shared rate limiter state
#[derive(Debug, Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, ClientBucket>>>,
    /// Maximum requests per minute per client
    max_rpm: u32,
}

impl RateLimiter {
    pub fn new(max_rpm: u32) -> Self {
        Self { buckets: Arc::new(Mutex::new(HashMap::new())), max_rpm }
    }

    /// Check if a request from the given client key is allowed.
    /// Returns true if allowed, false if rate-limited.
    async fn check(&self, client_key: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        // Periodic cleanup: remove entries older than 2 minutes
        // (done lazily, only when we have more than 10k entries)
        if buckets.len() > 10_000 {
            buckets.retain(|_, bucket| now.duration_since(bucket.window_start).as_secs() < 120);
        }

        let bucket =
            buckets.entry(client_key.to_string()).or_insert_with(|| ClientBucket { window_start: now, count: 0 });

        // Reset window if older than 60 seconds
        if now.duration_since(bucket.window_start).as_secs() >= 60 {
            bucket.window_start = now;
            bucket.count = 0;
        }

        bucket.count += 1;
        bucket.count <= self.max_rpm
    }
}

/// Extract client IP from request headers (X-Forwarded-For, X-Real-IP)
/// or fall back to a fixed key if no IP information is available.
fn extract_client_key(headers: &HeaderMap) -> String {
    // Try X-Forwarded-For first (standard proxy header)
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            // Take the first IP in the list (original client)
            if let Some(ip) = val.split(',').next() {
                let ip = ip.trim();
                if !ip.is_empty() {
                    return format!("ip:{ip}");
                }
            }
        }
    }

    // Try X-Real-IP (nginx, etc.)
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(val) = xri.to_str() {
            let ip = val.trim();
            if !ip.is_empty() {
                return format!("ip:{ip}");
            }
        }
    }

    // Fallback: use a generic key (will rate-limit all clients together)
    "unknown".to_string()
}

/// A tower/axum layer that applies rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitLayer {
    limiter: RateLimiter,
}

impl RateLimitLayer {
    pub fn new(max_rpm: u32) -> Self {
        Self { limiter: RateLimiter::new(max_rpm) }
    }

    pub fn limiter(&self) -> RateLimiter {
        self.limiter.clone()
    }
}

impl<S> tower::Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware { inner, limiter: self.limiter.clone() }
    }
}

/// The middleware service produced by [`RateLimitLayer`].
#[derive(Debug, Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    limiter: RateLimiter,
}

impl<S> tower::Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let limiter = self.limiter.clone();
        let client_key = extract_client_key(req.headers());
        let inner = self.inner.call(req);

        Box::pin(async move {
            if limiter.check(&client_key).await {
                inner.await
            } else {
                log::warn!("Rate limit exceeded for client: {client_key}");
                Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, "60")],
                    r#"{"error":"rate_limit_exceeded","message":"Too many requests. Please try again later."}"#,
                )
                    .into_response())
            }
        })
    }
}
