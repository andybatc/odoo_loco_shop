#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use crate::middleware::auth_extractor::AuthToken;

#[derive(Debug, Deserialize)]
pub struct StripeConfigPayload {
    pub stripe_secret_key: String,
}

#[debug_handler]
pub async fn sync_stripe_config(
    State(ctx): State<AppContext>,
    _auth: AuthToken,
    Json(payload): Json<StripeConfigPayload>,
) -> Result<Response> {
    let key = payload.stripe_secret_key.trim().to_string();

    if !key.starts_with("sk_") && !key.starts_with("rk_") {
        tracing::warn!("Stripe config received with unexpected key prefix (not sk_ or rk_)");
    }

    // Store in configs table (same pattern as views.rs:handle_config_update)
    let config = crate::models::_entities::configs::Entity::find()
        .filter(crate::models::_entities::configs::Column::Key.eq("stripe_secret_key"))
        .one(&ctx.db)
        .await?;
    if let Some(c) = config {
        let mut active_model: crate::models::_entities::configs::ActiveModel = c.into();
        active_model.value = Set(Some(key.clone()));
        active_model.update(&ctx.db).await?;
    } else {
        crate::models::_entities::configs::ActiveModel {
            key: Set(Some("stripe_secret_key".to_string())),
            value: Set(Some(key.clone())),
            ..Default::default()
        }.insert(&ctx.db).await?;
    }

    // Invalidate Redis cache
    crate::models::config_cache::invalidate_config_cache(&ctx, "stripe_secret_key").await;

    tracing::info!("Stripe secret key updated via webhook");

    format::json(serde_json::json!({
        "status": "ok"
    }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/webhooks/odoo")
        .add("/stripe-config", post(sync_stripe_config))
}
