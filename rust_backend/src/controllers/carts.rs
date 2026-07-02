#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::extract::{State, Json, Form};
use axum::extract::Path;

use loco_rs::auth::jwt::JWT;
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
    headers: axum::http::HeaderMap,
    Form(params): Form<AddItemParams>,
) -> Result<Response, Error> {

    tracing::info!("🛒 add_to_cart: product_id={}, cookie_header={:?}", params.product_id, headers.get("cookie").map(|h| h.to_str().ok()));

    let product = products::Entity::find_by_id(params.product_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::BadRequest("Producto no encontrado".to_string()))?;

    if let Some(stock) = product.stock {
        if stock <= 0.0 {
            return Err(Error::BadRequest("Producto sin stock disponible".to_string()));
        }
    }

    let current_user_id: Option<Uuid> = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_str| {
            let token = cookie_str
                .split(';')
                .find(|s| s.trim().starts_with("token="))?
                .split('=')
                .nth(1)?;
            let jwt_config = ctx.config.get_jwt_config().ok()?;
            let auth = JWT::new(&jwt_config.secret)
                .validate(token)
                .ok()?;
            let pid = Uuid::parse_str(&auth.claims.pid).ok()?;
            Some(pid)
        });

    let mut current_cart: Option<carts::Model> = None;

    let read_cookie = |name: &str| -> Option<String> {
        let cookie_str = headers.get("cookie")?.to_str().ok()?;
        cookie_str.split(';').find(|s| s.trim().starts_with(name))?.split('=').nth(1).map(|s| s.trim().to_string())
    };

    if let Some(uid) = current_user_id {
        current_cart = carts::Entity::find()
            .filter(carts::Column::UserId.eq(uid))
            .one(&ctx.db)
            .await?;
        tracing::info!("🛒 add_to_cart: user logged in uid={}, found_cart={:?}", uid, current_cart.as_ref().map(|c| c.id));
    } else if let Some(cookie_val) = read_cookie("rsv_cart_session") {
        if let Ok(parsed_uuid) = Uuid::parse_str(&cookie_val) {
            current_cart = carts::Entity::find_by_id(parsed_uuid).one(&ctx.db).await?;
            tracing::info!("🛒 add_to_cart: existing guest cookie={}, found_cart={:?}", parsed_uuid, current_cart.is_some());
        } else {
            tracing::warn!("🛒 add_to_cart: invalid cookie val={}", cookie_val);
        }
    } else {
        tracing::info!("🛒 add_to_cart: no user cookie, no rsv_cart_session cookie");
    }

    let cart = match current_cart {
        Some(c) => c,
        None => {
            let cart_id = Uuid::new_v4();
            tracing::info!("🛒 add_to_cart: creating new cart id={}, uid={:?}", cart_id, current_user_id);
            let new_cart = carts::ActiveModel {
                id: Set(cart_id),
                user_id: Set(current_user_id),
                ..Default::default()
            };
            new_cart.insert(&ctx.db).await?
        }
    };

    if let Some(uid) = current_user_id {
        if cart.user_id.is_none() {
            let mut active_cart: carts::ActiveModel = cart.clone().into();
            active_cart.user_id = Set(Some(uid));
            active_cart.update(&ctx.db).await?;
        }
    }

    let existing_item = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart.id))
        .filter(cart_items::Column::ProductId.eq(params.product_id))
        .one(&ctx.db)
        .await?;

    let current_qty = existing_item.as_ref().map(|i| i.quantity).unwrap_or(0);
    let new_qty = current_qty + 1;

    if let Some(stock) = product.stock {
        if new_qty as f32 > stock {
            return Err(Error::BadRequest(
                format!("Stock insuficiente. Disponible: {}, en tu carrito: {}", stock as i32, current_qty),
            ));
        }
    }

    if let Some(item) = existing_item {
        tracing::info!("🛒 add_to_cart: updating qty item={:?} from {} to {}", item.id, item.quantity, new_qty);
        let mut active_item: cart_items::ActiveModel = item.into();
        active_item.quantity = Set(new_qty);
        active_item.update(&ctx.db).await?;
    } else {
        let item_id = Uuid::new_v4();
        tracing::info!("🛒 add_to_cart: inserting item id={}, cart={}, product={}, qty={}",
            item_id, cart.id, params.product_id, new_qty);
        let new_item = cart_items::ActiveModel {
            id: Set(item_id),
            cart_id: Set(cart.id),
            product_id: Set(params.product_id),
            quantity: Set(new_qty),
            ..Default::default()
        };
        new_item.insert(&ctx.db).await?;
    }

    tracing::info!("🛒 add_to_cart: done, setting cookie={}", cart.id);
    let body = serde_json::json!({
        "status": "success",
        "message": "Producto agregado al carrito",
        "cart_id": cart.id
    });
    let bytes = serde_json::to_vec(&body).map_err(|e| Error::wrap(e))?;

    let mut response_builder = axum::response::Response::builder()
        .header("content-type", "application/json");
    if current_user_id.is_none() {
        response_builder = response_builder.header(
            "Set-Cookie",
            format!("rsv_cart_session={}; Path=/; HttpOnly; SameSite=Lax", cart.id),
        );
    }
    response_builder
        .body(axum::body::Body::from(bytes))
        .map_err(|e| Error::wrap(e))
}

fn read_cart_cookie(headers: &axum::http::HeaderMap) -> Option<Uuid> {
    let cookie_header = headers.get("cookie")?;
    let cookie_str = cookie_header.to_str().ok()?;
    let found = cookie_str.split(';').find(|s| s.trim().starts_with("rsv_cart_session="));
    tracing::info!("🛒 read_cart_cookie: raw_cookie={:?}, found_rsv={:?}", cookie_str, found);
    let val = found?.split('=').nth(1)?;
    let parsed = Uuid::parse_str(val.trim()).ok();
    tracing::info!("🛒 read_cart_cookie: val={:?}, parsed={:?}", val.trim(), parsed);
    parsed
}

async fn find_cart_from_cookie(headers: &axum::http::HeaderMap, ctx: &AppContext) -> Result<Option<carts::Model>> {
    if let Some(uuid) = read_cart_cookie(headers) {
        return carts::Entity::find_by_id(uuid)
            .one(&ctx.db)
            .await
            .map_err(|_| Error::string("Error al buscar carrito"));
    }
    Ok(None)
}

async fn find_cart(headers: &axum::http::HeaderMap, ctx: &AppContext) -> Result<Option<carts::Model>> {
    if let Some(uid) = get_user_id(headers, ctx) {
        tracing::info!("🛒 find_cart: logged-in user={}", uid);
        let cart = carts::Entity::find()
            .filter(carts::Column::UserId.eq(uid))
            .one(&ctx.db)
            .await
            .map_err(|_| Error::string("Error al buscar carrito"))?;
        if cart.is_some() {
            return Ok(cart);
        }
    }
    find_cart_from_cookie(headers, ctx).await
}

fn get_user_id(headers: &axum::http::HeaderMap, ctx: &AppContext) -> Option<Uuid> {
    let cookie_str = headers.get("cookie")?.to_str().ok()?;
    let token = cookie_str
        .split(';')
        .find(|s| s.trim().starts_with("token="))?
        .split('=')
        .nth(1)?;
    let jwt_config = ctx.config.get_jwt_config().ok()?;
    let auth = JWT::new(&jwt_config.secret).validate(token).ok()?;
    Uuid::parse_str(&auth.claims.pid).ok()
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
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<CartItemWithProduct>>> {
    tracing::info!("🛒 get_cart_items: cookie_header={:?}", headers.get("cookie").map(|h| h.to_str().ok()));
    let cart = find_cart(&headers, &ctx)
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
    headers: axum::http::HeaderMap,
    Path(item_id): Path<Uuid>,
    Json(params): Json<UpdateQuantityParams>,
) -> Result<Json<serde_json::Value>> {
    if params.quantity < 1 {
        return Err(Error::BadRequest("quantity must be at least 1".to_string()));
    }

    let cart = find_cart(&headers, &ctx)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    let item = cart_items::Entity::find()
        .filter(cart_items::Column::Id.eq(item_id))
        .filter(cart_items::Column::CartId.eq(cart.id))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)?;

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
    headers: axum::http::HeaderMap,
    Path(item_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let cart = find_cart(&headers, &ctx)
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
