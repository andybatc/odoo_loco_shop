#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use crate::workers::webhook::{WebhookWorker, WebhookWorkerArgs};
use crate::controllers::token_auth::AuthToken;
use std::time::Duration;

async fn check_rate_limit(ctx: &AppContext, key: &str, max: i64, window_secs: u64) -> Result<()> {
    let cache_key = format!("rate_limit:{}", key);

    let count = ctx
        .cache
        .get_or_insert_with_expiry::<i64, _>(
            &cache_key,
            Duration::from_secs(window_secs),
            async { Ok(0i64) },
        )
        .await?;

    if count >= max {
        return Err(Error::string("Límite de peticiones excedido"));
    }

    let _ = ctx
        .cache
        .insert_with_expiry(&cache_key, &(count + 1), Duration::from_secs(window_secs))
        .await;

    Ok(())
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

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/webhooks/odoo")
        .add("/update", post(update))
        .add("/bulk-update", post(update_bulk))
}
