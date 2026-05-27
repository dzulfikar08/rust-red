//! Security headers middleware
//!
//! Adds standard security headers to all HTTP responses:
//! - X-Content-Type-Options: nosniff
//! - X-Frame-Options: DENY
//! - X-XSS-Protection: 0 (disabled per modern guidance; CSP preferred)
//! - Referrer-Policy: strict-origin-when-cross-origin
//! - Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'
//! - Permissions-Policy: camera=(), microphone=(), geolocation=()
//!
//! Note: Strict-Transport-Security is NOT added by default since it should
//! only be set when HTTPS is in use (typically at a reverse-proxy level).

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, header};
use axum::middleware::Next;

/// Apply security headers to the response.
/// This is an axum middleware function intended to be used via `axum::middleware::from_fn`.
pub async fn add_security_headers(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();

    // Prevent MIME-type sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));

    // Prevent clickjacking - deny framing entirely
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Disable legacy XSS protection (browsers handle this better via CSP now)
    headers.insert(header::HeaderName::from_static("x-xss-protection"), HeaderValue::from_static("0"));

    // Control referrer information
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin"));

    // Content Security Policy - permissive for Node-RED editor compatibility
    // The editor requires inline scripts/styles and eval for the function editor
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:; \
             font-src 'self'; \
             connect-src 'self' ws: wss:; \
             worker-src 'self' blob:; \
             frame-ancestors 'none'",
        ),
    );

    // Disable unnecessary browser features
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );

    response
}

/// A tower/axum layer wrapper that applies security headers.
/// Usage: `.layer(SecurityHeadersLayer)`
#[derive(Debug, Clone, Copy)]
pub struct SecurityHeadersLayer;

impl<S> tower::Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersMiddleware { inner }
    }
}

/// The middleware service produced by [`SecurityHeadersLayer`].
#[derive(Debug, Clone)]
pub struct SecurityHeadersMiddleware<S> {
    inner: S,
}

impl<S> tower::Service<Request<Body>> for SecurityHeadersMiddleware<S>
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
        let fut = self.inner.call(req);
        Box::pin(async move {
            let mut response = fut.await?;
            let headers = response.headers_mut();

            headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
            headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
            headers.insert(header::HeaderName::from_static("x-xss-protection"), HeaderValue::from_static("0"));
            headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin"));
            headers.insert(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(
                    "default-src 'self'; \
                     script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                     style-src 'self' 'unsafe-inline'; \
                     img-src 'self' data: blob:; \
                     font-src 'self'; \
                     connect-src 'self' ws: wss:; \
                     worker-src 'self' blob:; \
                     frame-ancestors 'none'",
                ),
            );
            headers.insert(
                header::HeaderName::from_static("permissions-policy"),
                HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
            );

            Ok(response)
        })
    }
}
