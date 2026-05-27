use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;

use rust_red_core::runtime::audit::{AuditEventType, AuditLogger};

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            if let Some(ip) = val.split(',').next() {
                let ip = ip.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(val) = xri.to_str() {
            let ip = val.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }
    None
}

fn classify_request(req: &Request<Body>) -> (AuditEventType, serde_json::Value) {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match path.as_str() {
        p if p == "/flows" && method == "POST" => {
            (AuditEventType::FlowDeploy, serde_json::json!({"method": method.as_str(), "path": path}))
        }
        p if p.starts_with("/flow/") && method == "DELETE" => {
            (AuditEventType::FlowDelete, serde_json::json!({"method": method.as_str(), "path": path}))
        }
        p if p == "/nodes" && method == "POST" => {
            (AuditEventType::NodeCreate, serde_json::json!({"method": method.as_str(), "path": path}))
        }
        p if p.starts_with("/nodes/") && method == "DELETE" => {
            (AuditEventType::NodeDelete, serde_json::json!({"method": method.as_str(), "path": path}))
        }
        p if p.starts_with("/nodes/") && method == "PUT" => {
            (AuditEventType::PluginLoad, serde_json::json!({"method": method.as_str(), "path": path}))
        }
        _ => (AuditEventType::ConfigChange, serde_json::json!({"method": method.as_str(), "path": path})),
    }
}

#[derive(Clone)]
pub struct AuditLogLayer {
    logger: Arc<dyn AuditLogger>,
}

impl AuditLogLayer {
    pub fn new(logger: Arc<dyn AuditLogger>) -> Self {
        Self { logger }
    }
}

impl<S> tower::Layer<S> for AuditLogLayer {
    type Service = AuditLogMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuditLogMiddleware { inner, logger: self.logger.clone() }
    }
}

#[derive(Clone)]
pub struct AuditLogMiddleware<S> {
    inner: S,
    logger: Arc<dyn AuditLogger>,
}

impl<S> tower::Service<Request<Body>> for AuditLogMiddleware<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + Sync,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let logger = self.logger.clone();
        let ip = extract_ip(req.headers());
        let (event_type, details) = classify_request(&req);

        let user = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|_| "authenticated_user".to_string());

        let inner = self.inner.call(req);

        Box::pin(async move {
            let result = inner.await;

            let success = match &result {
                Ok(response) => {
                    let status = response.status();
                    !status.is_server_error() && status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN
                }
                Err(_) => false,
            };

            let event = rust_red_core::runtime::audit::AuditEvent::new(event_type).details(details).success(success);

            let event = match user {
                Some(u) => event.user(u),
                None => event,
            };
            let event = match ip {
                Some(ip) => event.ip_address(ip),
                None => event,
            };

            logger.log_event(event).await;

            result
        })
    }
}
