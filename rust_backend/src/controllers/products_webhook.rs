#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use crate::workers::webhook::{WebhookWorker, WebhookWorkerArgs};
use crate::models::configs;
use crate::controllers::token_auth::AuthToken;
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use std::time::Duration;
use axum::{
    error_handling::HandleErrorLayer,
    routing::post,
    Json,
    http::StatusCode,
    extract::State,
    http::HeaderMap
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
#[debug_handler]
pub async fn index(State(_ctx): State<AppContext>) -> Result<Response> {
    format::empty()
}

#[derive(Serialize, Deserialize)]
pub struct OdooPayload {
    pub odoo_id: i32,
}

async fn handle_rate_limit_error(err: BoxError) -> (StatusCode, String) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        format!("Límite de peticiones excedido: {}", err),
    )
}

#[axum::debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    _: AuthToken, // 👈 Se autentica automáticamente antes de entrar aquí
    Json(args): Json<WebhookWorkerArgs>
) -> Result<Response> {
    // Si llega aquí, el token es válido. ¡Ni una línea de código extra!
    WebhookWorker::perform_later(&ctx, args).await?;

    format::json::<()>(())
}

pub async fn update_bulk(
    State(ctx): State<AppContext>,
    _: AuthToken,
    Json(args_list): Json<Vec<WebhookWorkerArgs>> // args_list es propiedad de esta función
) -> Result<Response> {

    for args in args_list {
        // Ahora 'args' es de tipo 'WebhookWorkerArgs' (owned), no '&WebhookWorkerArgs'
        WebhookWorker::perform_later(&ctx, args).await?;
    }

    format::json(serde_json::json!({"status": "success"}))
}

pub fn routes() -> Routes {
    // Al añadir BufferLayer, hacemos que todo el middleware sea "Clone"
    // y Axum por fin lo aceptará felizmente.
    let middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(handle_rate_limit_error))
        .layer(BufferLayer::new(1024)) 
        .layer(RateLimitLayer::new(10, Duration::from_secs(1)));

    Routes::new()
        .prefix("api/webhooks/odoo")
        .add("/update", post(update))
        .add("/bulk-update", post(update_bulk))
        .layer(middleware)
}