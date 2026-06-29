use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};

pub async fn add_security_headers(
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();

    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "X-Frame-Options",
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("0"),
    );
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' https://unpkg.com; style-src 'self' 'unsafe-inline' https://unpkg.com; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';"),
    );

    response
}
