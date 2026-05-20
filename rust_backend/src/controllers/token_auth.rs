#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use loco_rs::prelude::*;
use crate::models::_entities::configs::{Entity as Configs, Column as ConfigColumn};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

pub struct AuthToken;

// 1. Eliminamos el #[async_trait]
impl<S> FromRequestParts<S> for AuthToken
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    // 2. Usamos async fn directamente. Rust inferirá los lifetimes correctamente.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);

        let auth_header = parts.headers.get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or_else(|| loco_rs::Error::Unauthorized("Token faltante".to_string()))?;

        let config = Configs::find()
            .filter(ConfigColumn::Key.eq("webhook_token"))
            .one(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::wrap(e))?
            .ok_or_else(|| loco_rs::Error::NotFound)?;

        if config.value.as_deref() != Some(auth_header) {
            return Err(loco_rs::Error::Unauthorized("Token inválido".to_string()));
        }

        Ok(AuthToken)
    }
}

pub async fn handle_webhook(
    State(_ctx): State<AppContext>,
    _: AuthToken,
    Json(_args): Json<serde_json::Value>,
) -> Result<Response> {
    format::json(serde_json::json!({"status": "success"}))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/token_auth")
        .add("/", post(handle_webhook))
}
