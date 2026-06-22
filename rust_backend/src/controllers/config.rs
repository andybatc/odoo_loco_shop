#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::models::_entities::configs;
use crate::models::config_cache;
use sea_orm::Set;
use regex::Regex;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OdooUrlRequest {
    pub url: String,
}

#[utoipa::path(
    get,
    path = "/api/config/token",
    responses(
        (status = 200, description = "Current webhook token", content_type = "text/plain")
    ),
    tag = "Config"
)]
pub(crate) async fn get_token(State(ctx): State<AppContext>) -> Result<Response> {
    let token_value = config_cache::get_cached_config(&ctx, "webhook_token")
        .await?
        .unwrap_or_else(|| "not_set".to_string());

    format::json(token_value)
}

#[utoipa::path(
    post,
    path = "/api/config/token",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token updated"),
        (status = 400, description = "Invalid token (min 8 chars)")
    ),
    tag = "Config"
)]
pub(crate) async fn update_token(
    State(ctx): State<AppContext>,
    Json(payload): Json<TokenRequest>,
) -> Result<Response> {
    if payload.token.len() < 8 {
        return Err(Error::BadRequest("token must be at least 8 characters".to_string()));
    }
    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq("webhook_token"))
        .one(&ctx.db)
        .await?;

    if let Some(c) = config {
        let mut active_model: configs::ActiveModel = c.into();
        active_model.value = Set(Some(payload.token));
        active_model.update(&ctx.db).await?;
    } else {
        configs::ActiveModel {
            key: Set(Some("webhook_token".to_string())),
            value: Set(Some(payload.token)),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await?;
    }

    config_cache::invalidate_config_cache(&ctx, "webhook_token").await;
    format::json(serde_json::json!({ "status": "ok" }))
}

#[utoipa::path(
    get,
    path = "/api/config/odoo-url",
    responses(
        (status = 200, description = "Current Odoo base URL", content_type = "text/plain")
    ),
    tag = "Config"
)]
pub(crate) async fn get_odoo_url(State(ctx): State<AppContext>) -> Result<Response> {
    let url_value = config_cache::get_cached_config(&ctx, "odoo_base_url")
        .await?
        .unwrap_or_else(|| "http://localhost:8072".to_string());

    format::json(url_value)
}

#[utoipa::path(
    post,
    path = "/api/config/odoo-url",
    request_body = OdooUrlRequest,
    responses(
        (status = 200, description = "URL updated"),
        (status = 400, description = "Invalid URL format")
    ),
    tag = "Config"
)]
pub(crate) async fn update_odoo_url(
    State(ctx): State<AppContext>,
    Json(payload): Json<OdooUrlRequest>,
) -> Result<Response> {
    let url_re = Regex::new(r"^https?://[a-zA-Z0-9][-a-zA-Z0-9.]*[a-zA-Z0-9](:[0-9]+)?(/.*)?$").unwrap();
    if !url_re.is_match(&payload.url) {
        return Err(Error::BadRequest("invalid URL format (must start with http:// or https://)".to_string()));
    }
    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq("odoo_base_url"))
        .one(&ctx.db)
        .await?;

    if let Some(c) = config {
        let mut active_model: configs::ActiveModel = c.into();
        active_model.value = Set(Some(payload.url));
        active_model.update(&ctx.db).await?;
    } else {
        configs::ActiveModel {
            key: Set(Some("odoo_base_url".to_string())),
            value: Set(Some(payload.url)),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await?;
    }

    config_cache::invalidate_config_cache(&ctx, "odoo_base_url").await;
    format::json(serde_json::json!({ "status": "ok" }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/config")
        .add("/token", get(get_token))
        .add("/token", post(update_token))
        .add("/odoo-url", get(get_odoo_url))
        .add("/odoo-url", post(update_odoo_url))
}
