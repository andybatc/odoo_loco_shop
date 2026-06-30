#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use crate::middleware::auth_extractor::AuthToken;
use crate::models::_entities::payment_methods;

#[derive(Debug, Deserialize)]
pub struct PaymentMethodPayload {
    pub odoo_provider_id: i32,
    pub name: String,
    pub code: String,
    pub state: String,
    pub is_published: bool,
    pub allow_tokenization: bool,
    pub capture_manually: bool,
    pub sequence: i32,
}

#[debug_handler]
pub async fn sync_payment_methods(
    State(ctx): State<AppContext>,
    _: AuthToken,
    Json(payloads): Json<Vec<PaymentMethodPayload>>,
) -> Result<Response> {
    for p in &payloads {
        let is_active = p.state == "enabled" || p.state == "test";

        let existing = payment_methods::Entity::find()
            .filter(payment_methods::Column::OdooProviderId.eq(p.odoo_provider_id))
            .one(&ctx.db)
            .await?;

        if let Some(record) = existing {
            let mut active: payment_methods::ActiveModel = record.into();
            active.name = Set(p.name.clone());
            active.code = Set(p.code.clone());
            active.is_active = Set(is_active);
            active.is_published = Set(p.is_published);
            active.allow_tokenization = Set(p.allow_tokenization);
            active.capture_manually = Set(p.capture_manually);
            active.sequence = Set(p.sequence);
            active.update(&ctx.db).await?;
        } else {
            payment_methods::ActiveModel {
                odoo_provider_id: Set(p.odoo_provider_id),
                name: Set(p.name.clone()),
                code: Set(p.code.clone()),
                is_active: Set(is_active),
                is_published: Set(p.is_published),
                allow_tokenization: Set(p.allow_tokenization),
                capture_manually: Set(p.capture_manually),
                sequence: Set(p.sequence),
                ..Default::default()
            }
            .insert(&ctx.db)
            .await?;
        }
    }

    let _ = ctx.cache.remove("payment_methods:all").await;

    format::json(serde_json::json!({
        "status": "ok",
        "synced": payloads.len()
    }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/webhooks/odoo")
        .add("/payment-methods", post(sync_payment_methods))
}
