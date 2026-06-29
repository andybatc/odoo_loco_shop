#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use crate::workers::webhook::{WebhookWorker, WebhookWorkerArgs};
use crate::middleware::auth_extractor::AuthToken;
use crate::models::_entities::users;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use loco_rs::config::CacheConfig;

pub(crate) async fn check_rate_limit(ctx: &AppContext, key: &str, max: i64, window_secs: u64) -> Result<()> {
    let redis_uri = match &ctx.config.cache {
        CacheConfig::Redis(cfg) => cfg.uri.clone(),
        _ => "redis://127.0.0.1:6379".to_string(),
    };

    let client = redis::Client::open(redis_uri.as_str())
        .map_err(Error::msg)?;
    let mut conn = client
        .get_multiplexed_tokio_connection()
        .await
        .map_err(Error::msg)?;

    let cache_key = format!("rate_limit:{key}");
    let count: i64 = redis::cmd("INCR")
        .arg(&cache_key)
        .query_async(&mut conn)
        .await
        .map_err(Error::msg)?;

    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&cache_key)
            .arg(window_secs)
            .query_async(&mut conn)
            .await
            .map_err(Error::msg)?;
    }

    if count > max {
        Err(Error::string("Límite de peticiones excedido"))
    } else {
        Ok(())
    }
}

#[utoipa::path(
    post,
    path = "/api/webhooks/odoo/update",
    request_body = WebhookWorkerArgs,
    responses(
        (status = 200, description = "Product update queued"),
        (status = 401, description = "Invalid webhook token"),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Webhooks"
)]
#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    _: AuthToken,
    Json(args): Json<WebhookWorkerArgs>,
) -> Result<Response> {
    check_rate_limit(&ctx, "webhook:update", 10, 1).await?;

    WebhookWorker::perform_later(&ctx, args).await?;
    format::json::<()>(())
}

#[utoipa::path(
    post,
    path = "/api/webhooks/odoo/bulk-update",
    request_body = Vec<WebhookWorkerArgs>,
    responses(
        (status = 200, description = "Bulk update queued"),
        (status = 401, description = "Invalid webhook token"),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Webhooks"
)]
pub async fn update_bulk(
    State(ctx): State<AppContext>,
    _: AuthToken,
    Json(args_list): Json<Vec<WebhookWorkerArgs>>,
) -> Result<Response> {
    check_rate_limit(&ctx, "webhook:bulk-update", 5, 1).await?;

    for args in args_list {
        WebhookWorker::perform_later(&ctx, args).await?;
    }

    format::json(serde_json::json!({"status": "success"}))
}

#[derive(Debug, Deserialize)]
pub struct AdminWebhookPayload {
    pub email: String,
    pub action: String,
}

#[debug_handler]
pub async fn admin_webhook(
    State(ctx): State<AppContext>,
    _: AuthToken,
    Json(payload): Json<AdminWebhookPayload>,
) -> Result<Response> {
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| {
            tracing::warn!("admin webhook: user {} not found", payload.email);
            Error::NotFound
        })?;

    match payload.action.as_str() {
        "promote" => {
            let mut active: users::ActiveModel = user.into();
            active.role = Set("admin".to_string());
            active.update(&ctx.db).await?;
            tracing::info!("User {} promoted to admin via webhook", payload.email);
        }
        "demote" => {
            let mut active: users::ActiveModel = user.into();
            active.role = Set("user".to_string());
            active.update(&ctx.db).await?;
            tracing::info!("User {} demoted to user via webhook", payload.email);
        }
        _ => return Err(Error::BadRequest("action must be 'promote' or 'demote'".to_string())),
    }

    format::json(serde_json::json!({"status": "ok"}))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/webhooks/odoo")
        .add("/update", post(update))
        .add("/bulk-update", post(update_bulk))
        .add("/admin", post(admin_webhook))
}
