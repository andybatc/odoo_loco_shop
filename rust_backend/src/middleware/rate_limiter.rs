#![allow(clippy::missing_errors_doc)]

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use loco_rs::app::AppContext;
use loco_rs::config::CacheConfig;
use std::sync::OnceLock;

static REDIS_CLIENT: OnceLock<redis::Client> = OnceLock::new();

fn classify_route(method: &Method, path: &str) -> Option<(&'static str, i64, u64)> {
    match *method {
        Method::POST => match path {
            "/api/auth/register"
            | "/api/auth/login"
            | "/api/auth/forgot"
            | "/api/auth/reset"
            | "/api/auth/magic-link"
            | "/api/auth/resend-verification-mail" => Some(("auth", 10, 60)),
            "/ui/auth/web-login" => Some(("web_login", 10, 60)),
            "/api/carts/" => Some(("carts_write", 30, 60)),
            p if p.starts_with("/api/carts/items/") && p.len() > "/api/carts/items/".len() => {
                Some(("carts_write", 30, 60))
            }
            _ => None,
        },
        Method::GET => match path {
            p if p.starts_with("/api/auth/magic-link/") => Some(("auth", 10, 60)),
            p if p.starts_with("/api/auth/verify/") => Some(("verify", 10, 60)),
            "/api/carts/items" => Some(("carts_read", 30, 60)),
            "/shop/api/search" => Some(("search", 30, 60)),
            "/api/shipping/estimate" => Some(("shipping", 30, 60)),
            p if p.starts_with("/shop/api/product/") => Some(("product_api", 30, 60)),
            _ => None,
        },
        Method::DELETE => match path {
            p if p.starts_with("/api/carts/items/") => Some(("carts_write", 30, 60)),
            _ => None,
        },
        _ => None,
    }
}

fn should_skip(path: &str) -> bool {
    path.starts_with("/static/")
        || path.starts_with("/storage/")
        || path.starts_with("/admin/")
        || path.starts_with("/swagger-ui")
        || path.starts_with("/api-docs")
        || path == "/metrics"
        || path.starts_with("/api/webhooks/")
}

fn extract_ip(req: &Request<Body>) -> String {
    req.headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            req.headers()
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn rate_limit_middleware(
    State(ctx): State<AppContext>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    if should_skip(&path) {
        return next.run(req).await;
    }

    let Some((group, max_requests, window_secs)) = classify_route(req.method(), &path) else {
        return next.run(req).await;
    };

    let ip = extract_ip(&req);

    // Lazily initialize the Redis client from app config
    let client = REDIS_CLIENT.get_or_init(|| {
        let uri = match &ctx.config.cache {
            CacheConfig::Redis(cfg) => cfg.uri.clone(),
            _ => "redis://127.0.0.1:6379".to_string(),
        };
        redis::Client::open(uri.as_str())
            .expect("Failed to create Redis client for rate limiter")
    });

    let mut conn = match client.get_multiplexed_tokio_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::warn!("rate_limiter: Redis connection error: {:?}", e);
            return next.run(req).await;
        }
    };

    let cache_key = format!("rl:{}:{}", group, ip);
    let count: i64 = match redis::cmd("INCR")
        .arg(&cache_key)
        .query_async(&mut conn)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("rate_limiter: Redis INCR error: {:?}", e);
            return next.run(req).await;
        }
    };

    if count == 1 {
        // EXPIRE errors are non-fatal — key will expire eventually or be cleaned up
        let _ = redis::cmd("EXPIRE")
            .arg(&cache_key)
            .arg(window_secs)
            .query_async::<()>(&mut conn)
            .await;
    }

    if count > max_requests {
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "text/plain; charset=utf-8")
            .header("Retry-After", window_secs.to_string())
            .body(Body::from(
                "Demasiadas solicitudes. Intenta de nuevo más tarde.",
            ))
            .unwrap();
    }

    next.run(req).await
}
