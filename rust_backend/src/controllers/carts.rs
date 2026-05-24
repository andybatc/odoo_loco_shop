#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::{extract::State, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::_entities::{cart_items, carts};

#[derive(Debug, Deserialize)]
pub struct AddItemParams {
    pub product_id: i32,
}

pub async fn add_to_cart(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Json(params): Json<AddItemParams>,
) -> Result<(CookieJar, Json<serde_json::Value>), Error> {

    let mut current_user_id: Option<Uuid> = None;

    if let Some(_auth_header) = headers.get("Authorization") {
        // TODO: Decodificar JWT si se requiere en el futuro
    }

    let mut current_cart: Option<carts::Model> = None;
    let cookie_name = "rsv_cart_session";

    // 1. Buscar el carrito existente
    if let Some(uid) = current_user_id {
        current_cart = carts::Entity::find()
            .filter(carts::Column::UserId.eq(uid))
            .one(&ctx.db)
            .await?;
    } else if let Some(cookie) = jar.get(cookie_name) {
        if let Ok(parsed_uuid) = Uuid::parse_str(cookie.value()) {
            current_cart = carts::Entity::find_by_id(parsed_uuid).one(&ctx.db).await?;
        }
    }

    // 2. Crear el carrito si no existe alguno activo
    let cart = match current_cart {
        Some(c) => c,
        None => {
            let new_cart = carts::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(current_user_id),
                ..Default::default()
            };
            new_cart.insert(&ctx.db).await?
        }
    };

    // 3. Adaptar el i32 a un Uuid válido para Postgres mediante padding hexadecimal
    let padded_hex = format!("{:032x}", params.product_id);
    let target_product_uuid = Uuid::parse_str(&padded_hex).unwrap_or_else(|_| Uuid::nil());

    // 4. Buscar si el ítem ya existe en el carrito
    let existing_item = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart.id))
        .filter(cart_items::Column::ProductId.eq(target_product_uuid))
        .one(&ctx.db)
        .await?;

    if let Some(item) = existing_item {
        // Si ya existe, incrementamos la cantidad
        let mut active_item: cart_items::ActiveModel = item.into();
        active_item.quantity = Set(active_item.quantity.unwrap() + 1);
        active_item.update(&ctx.db).await?;
    } else {
        // Si es nuevo, lo insertamos con cantidad inicial de 1
        let new_item = cart_items::ActiveModel {
            id: Set(Uuid::new_v4()),
            cart_id: Set(cart.id),
            product_id: Set(target_product_uuid),
            quantity: Set(1),
            ..Default::default()
        };
        new_item.insert(&ctx.db).await?;
    }

    // 5. Gestión de la cookie para usuarios invitados
    let mut response_jar = jar;
    if current_user_id.is_none() {
        let cookie = Cookie::build((cookie_name, cart.id.to_string()))
            .path("/")
            .http_only(true)
            .build();
        response_jar = response_jar.add(cookie);
    }

    Ok((
        response_jar,
        Json(serde_json::json!({
            "status": "success",
            "message": "Producto agregado al carrito",
            "cart_id": cart.id
        })),
    ))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/carts")
        .add("/", post(add_to_cart))
}
