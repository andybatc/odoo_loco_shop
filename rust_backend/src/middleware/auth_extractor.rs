#![allow(clippy::missing_errors_doc)]
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use loco_rs::auth::jwt::JWT;
use loco_rs::prelude::*;

use crate::models::_entities::users;
use crate::models::_entities::configs as configs_entity;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub struct CurrentUser {
    pub user: users::Model,
}

pub struct AdminUser {
    pub user: users::Model,
}

impl<S> FromRequestParts<S> for CurrentUser
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);

        let token = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .or_else(|| {
                parts
                    .headers
                    .get("cookie")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|cookie_str| {
                        cookie_str
                            .split(';')
                            .find(|s| s.trim().starts_with("token="))
                            .and_then(|s| s.split('=').nth(1))
                    })
            })
            .ok_or_else(|| {
                loco_rs::Error::Unauthorized("No autenticado".to_string())
            })?;

        let jwt_config = ctx.config.get_jwt_config()?;

        let auth = JWT::new(&jwt_config.secret).validate(token).map_err(|_| {
            loco_rs::Error::Unauthorized("Token inválido".to_string())
        })?;

        let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
            .await
            .map_err(|_| {
                loco_rs::Error::Unauthorized("Usuario no encontrado".to_string())
            })?;

        Ok(CurrentUser { user })
    }
}

impl<S> FromRequestParts<S> for AdminUser
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let current = CurrentUser::from_request_parts(parts, state).await?;

        if current.user.role != "admin" {
            return Err(loco_rs::Error::Unauthorized(
                "Se requiere rol administrador".to_string(),
            ));
        }

        Ok(AdminUser {
            user: current.user,
        })
    }
}

pub struct AuthToken;

impl<S> FromRequestParts<S> for AuthToken
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = loco_rs::Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);

        let auth_header = parts.headers.get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or_else(|| loco_rs::Error::Unauthorized("Token faltante".to_string()))?;

        let config = configs_entity::Entity::find()
            .filter(configs_entity::Column::Key.eq("webhook_token"))
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
