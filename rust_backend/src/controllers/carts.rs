#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::{extract::State, Json};
use axum::extract::Path;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::_entities::{cart_items, carts, products};

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

    // 0. Validar que el producto exista y tenga stock
    let product = products::Entity::find_by_id(params.product_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::BadRequest("Producto no encontrado".to_string()))?;

    if let Some(stock) = product.stock {
        if stock <= 0.0 {
            return Err(Error::BadRequest("Producto sin stock disponible".to_string()));
        }
    }

    let current_user_id: Option<Uuid> = None;

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

    // 3. Buscar si el ítem ya existe en el carrito
    let existing_item = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart.id))
        .filter(cart_items::Column::ProductId.eq(params.product_id))
        .one(&ctx.db)
        .await?;

    let current_qty = existing_item.as_ref().map(|i| i.quantity).unwrap_or(0);
    let new_qty = current_qty + 1;

    // 4. Validar stock con la nueva cantidad total
    if let Some(stock) = product.stock {
        if new_qty as f32 > stock {
            return Err(Error::BadRequest(
                format!("Stock insuficiente. Disponible: {}, en tu carrito: {}", stock as i32, current_qty),
            ));
        }
    }

    if let Some(item) = existing_item {
        // Si ya existe, incrementamos la cantidad
        let mut active_item: cart_items::ActiveModel = item.into();
        active_item.quantity = Set(new_qty);
        active_item.update(&ctx.db).await?;
    } else {
        // Si es nuevo, lo insertamos con cantidad inicial de 1
        let new_item = cart_items::ActiveModel {
            id: Set(Uuid::new_v4()),
            cart_id: Set(cart.id),
            product_id: Set(params.product_id),
            quantity: Set(new_qty),
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
            .same_site(axum_extra::extract::cookie::SameSite::Lax)
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

async fn find_cart_from_cookie(jar: &CookieJar, ctx: &AppContext) -> Result<Option<carts::Model>> {
    let cookie_name = "rsv_cart_session";
    if let Some(cookie) = jar.get(cookie_name) {
        if let Ok(parsed_uuid) = Uuid::parse_str(cookie.value()) {
            return carts::Entity::find_by_id(parsed_uuid)
                .one(&ctx.db)
                .await
                .map_err(|_| Error::string("Error al buscar carrito"));
        }
    }
    Ok(None)
}

#[derive(Serialize, ToSchema)]
pub struct CartItemWithProduct {
    pub item_id: Uuid,
    pub product_id: i32,
    pub product_name: String,
    pub price: String,
    pub quantity: i32,
    pub subtotal: String,
    pub image_filename: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/carts/items",
    responses(
        (status = 200, description = "Cart items list", body = Vec<CartItemWithProduct>),
        (status = 404, description = "Cart not found")
    ),
    tag = "Cart"
)]
pub async fn get_cart_items(
    State(ctx): State<AppContext>,
    jar: CookieJar,
) -> Result<Json<Vec<CartItemWithProduct>>> {
    let cart = find_cart_from_cookie(&jar, &ctx)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    let items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart.id))
        .all(&ctx.db)
        .await?;

    if items.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(product_ids))
        .all(&ctx.db)
        .await?;

    let prod_map: std::collections::HashMap<i32, &products::Model> =
        db_products.iter().map(|p| (p.id, p)).collect();

    let result: Vec<CartItemWithProduct> = items
        .into_iter()
        .map(|item| {
            let prod = prod_map.get(&item.product_id);
            let price_str = prod
                .and_then(|p| p.price)
                .map(|d| d.to_string())
                .unwrap_or_else(|| "0.00".to_string());
            let price_val: f64 = price_str.parse().unwrap_or(0.0);
            let subtotal = price_val * item.quantity as f64;
            CartItemWithProduct {
                item_id: item.id,
                product_id: item.product_id,
                product_name: prod
                    .and_then(|p| p.name.clone())
                    .unwrap_or_else(|| "Producto".to_string()),
                price: price_str,
                quantity: item.quantity,
                subtotal: format!("{:.2}", subtotal),
                image_filename: prod.and_then(|p| p.image_filename.clone()),
            }
        })
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateQuantityParams {
    pub quantity: i32,
}

#[utoipa::path(
    post,
    path = "/api/carts/items/{item_id}",
    params(
        ("item_id" = Uuid, Path, description = "Cart item UUID")
    ),
    request_body = UpdateQuantityParams,
    responses(
        (status = 200, description = "Quantity updated"),
        (status = 400, description = "Invalid quantity"),
        (status = 404, description = "Cart or item not found")
    ),
    tag = "Cart"
)]
pub async fn update_cart_item_quantity(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    Path(item_id): Path<Uuid>,
    Json(params): Json<UpdateQuantityParams>,
) -> Result<Json<serde_json::Value>> {
    if params.quantity < 1 {
        return Err(Error::BadRequest("quantity must be at least 1".to_string()));
    }

    let cart = find_cart_from_cookie(&jar, &ctx)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    let item = cart_items::Entity::find()
        .filter(cart_items::Column::Id.eq(item_id))
        .filter(cart_items::Column::CartId.eq(cart.id))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    // Validar stock antes de actualizar
    let product = products::Entity::find_by_id(item.product_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    if let Some(stock) = product.stock {
        if params.quantity as f32 > stock {
            return Err(Error::BadRequest(
                format!("Stock insuficiente. Disponible: {}", stock as i32),
            ));
        }
    }

    let mut active_item: cart_items::ActiveModel = item.into();
    active_item.quantity = Set(params.quantity);
    active_item.update(&ctx.db).await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

#[utoipa::path(
    delete,
    path = "/api/carts/items/{item_id}",
    params(
        ("item_id" = Uuid, Path, description = "Cart item UUID")
    ),
    responses(
        (status = 200, description = "Item removed"),
        (status = 404, description = "Cart or item not found")
    ),
    tag = "Cart"
)]
pub async fn remove_cart_item(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    Path(item_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let cart = find_cart_from_cookie(&jar, &ctx)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    let item = cart_items::Entity::find()
        .filter(cart_items::Column::Id.eq(item_id))
        .filter(cart_items::Column::CartId.eq(cart.id))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    cart_items::Entity::delete_by_id(item.id)
        .exec(&ctx.db)
        .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/carts")
        .add("/", post(add_to_cart))
        .add("/items", get(get_cart_items))
        .add("/items/{id}", post(update_cart_item_quantity))
        .add("/items/{id}", delete(remove_cart_item))
}
