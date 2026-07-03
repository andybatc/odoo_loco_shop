#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use crate::controllers::views::get_current_user;
use crate::models::_entities::{cart_items, carts, order_items, products};
use crate::models::_entities::orders as orders_entity;
use crate::models::_entities::payment_methods as payment_methods_entity;
use crate::models::cart_helpers;
use crate::workers::order_creation::{OrderCreationWorker, OrderWorkerArgs};
use axum::extract::Query;
use axum::http::HeaderMap;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CustomerInfo {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub zip: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CheckoutRequest {
    pub customer: CustomerInfo,
    pub payment_method_id: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct CheckoutResponse {
    pub success: bool,
    pub order_name: Option<String>,
    pub invoice_name: Option<String>,
    pub total: Option<f64>,
    pub error: Option<String>,
}

pub async fn checkout_page(
    State(ctx): State<AppContext>,
    ViewEngine(v): ViewEngine<TeraView>,
    headers: HeaderMap,
) -> Result<Response> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    let (items, total) = if let Some(ref u) = user {
        tracing::info!("🧾 checkout_page: usuario logueado {}, buscando carrito por user_id", u.pid);
        let cart = carts::Entity::find()
            .filter(carts::Column::UserId.eq(u.pid))
            .one(&ctx.db)
            .await?;
        match cart {
            Some(c) => {
                let loaded = cart_helpers::load_cart(&ctx, c.id).await?;
                tracing::info!("🧾 checkout_page: carrito encontrado, {} items", loaded.items.len());
                (loaded.items, loaded.total)
            }
            None => {
                tracing::info!("🧾 checkout_page: usuario sin carrito");
                (vec![], 0.0)
            }
        }
    } else {
        let cookie_val = headers
            .get("cookie")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(';').find(|p| p.trim().starts_with("rsv_cart_session=")))
            .and_then(|s| s.split('=').nth(1).map(|v| v.trim().to_string()));
        match cookie_val.and_then(|v| Uuid::parse_str(&v).ok()) {
            Some(cart_uuid) => {
                let cart = cart_helpers::load_cart(&ctx, cart_uuid).await?;
                (cart.items, cart.total)
            }
            None => (vec![], 0.0),
        }
    };

    let cached_methods: Option<Vec<payment_methods_entity::Model>> =
        ctx.cache.get("payment_methods:all").await.ok().flatten();
    let payment_methods = if let Some(methods) = cached_methods {
        methods
    } else {
        let methods = payment_methods_entity::Entity::find()
            .filter(payment_methods_entity::Column::IsActive.eq(true))
            .order_by_asc(payment_methods_entity::Column::Sequence)
            .all(&ctx.db)
            .await
            .unwrap_or_default();
        let _ = ctx
            .cache
            .insert_with_expiry("payment_methods:all", &methods, Duration::from_secs(300))
            .await;
        methods
    };

    format::render().view(
        &v,
        "shop/checkout.html",
        &serde_json::json!({
            "items": items,
            "total": total,
            "current_user": user,
            "payment_methods": payment_methods,
        }),
    )
}

#[utoipa::path(
    post,
    path = "/api/checkout",
    request_body = CheckoutRequest,
    responses(
        (status = 200, description = "Checkout result", body = CheckoutResponse)
    ),
    tag = "Checkout"
)]
pub(crate) async fn submit_checkout(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(params): Json<CheckoutRequest>,
) -> Result<(CookieJar, Json<CheckoutResponse>)> {
    let email_re = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    if !email_re.is_match(&params.customer.email) {
        return Ok((
            jar,
            Json(CheckoutResponse {
                success: false,
                order_name: None,
                invoice_name: None,
                total: None,
                error: Some("Email inválido".to_string()),
            }),
        ));
    }

    if params.customer.name.trim().is_empty() {
        return Ok((
            jar,
            Json(CheckoutResponse {
                success: false,
                order_name: None,
                invoice_name: None,
                total: None,
                error: Some("El nombre es obligatorio".to_string()),
            }),
        ));
    }

    let cookie_name = "rsv_cart_session";

    let cart_uuid = {
        let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
        let user = get_current_user(&ctx, cookie_header.map(|s| s.to_string())).await;
        if let Some(ref u) = user {
            let cart = carts::Entity::find()
                .filter(carts::Column::UserId.eq(u.pid))
                .one(&ctx.db)
                .await?;
            match cart {
                Some(c) => c.id,
                None => {
                    tracing::info!("🧾 submit_checkout: usuario logueado sin carrito");
                    return Ok((jar, Json(CheckoutResponse {
                        success: false, order_name: None, invoice_name: None, total: None,
                        error: Some("Carrito no encontrado".to_string()),
                    })));
                }
            }
        } else {
            let cookie_val = headers.get("cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(';').find(|p| p.trim().starts_with("rsv_cart_session=")))
                .and_then(|s| s.split('=').nth(1).map(|v| v.trim().to_string()));
            match cookie_val.and_then(|v| Uuid::parse_str(&v).ok()) {
                Some(id) => id,
                None => {
                    return Ok((jar, Json(CheckoutResponse {
                        success: false, order_name: None, invoice_name: None, total: None,
                        error: Some("Carrito no encontrado".to_string()),
                    })));
                }
            }
        }
    };
    tracing::info!("🧾 submit_checkout: cart_uuid={}, email={}", cart_uuid, params.customer.email);

    let items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .all(&ctx.db)
        .await?;

    if items.is_empty() {
        return Ok((
            jar,
            Json(CheckoutResponse {
                success: false,
                order_name: None,
                invoice_name: None,
                total: None,
                error: Some("El carrito está vacío".to_string()),
            }),
        ));
    }

    let mut product_ids = Vec::new();
    let mut item_map = std::collections::HashMap::new();
    for item in &items {
        product_ids.push(item.product_id);
        item_map.insert(item.product_id, item.quantity);
    }

    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(product_ids))
        .all(&ctx.db)
        .await
        ?;

    let mut total = sea_orm::prelude::Decimal::ZERO;
    for prod in &db_products {
        let qty = *item_map.get(&prod.id).unwrap_or(&1);
        let price = prod.price.unwrap_or(sea_orm::prelude::Decimal::ZERO);
        let subtotal = price * sea_orm::prelude::Decimal::from(qty as i64);
        total += subtotal;
    }

    let total_f64 = total.to_string().parse::<f64>().unwrap_or(0.0);

    let order_id = Uuid::new_v4();
    let order = orders_entity::ActiveModel {
        id: Set(order_id),
        customer_name: Set(params.customer.name.clone()),
        customer_email: Set(params.customer.email.clone()),
        customer_phone: Set(params.customer.phone.clone()),
        customer_street: Set(params.customer.street.clone()),
        customer_city: Set(params.customer.city.clone()),
        customer_zip: Set(params.customer.zip.clone()),
        total: Set(total),
        status: Set("pending".to_string()),
        ..Default::default()
    };
    order.insert(&ctx.db).await?;

    let order_items_to_insert: Vec<order_items::ActiveModel> = db_products.iter().map(|prod| {
        let qty = *item_map.get(&prod.id).unwrap_or(&1);
        let price = prod.price.unwrap_or(sea_orm::prelude::Decimal::ZERO);
        let subtotal = price * sea_orm::prelude::Decimal::from(qty as i64);
        order_items::ActiveModel {
            id: Set(Uuid::new_v4()),
            order_id: Set(order_id),
            product_id: Set(prod.id),
            product_name: Set(prod.name.clone().unwrap_or_else(|| "Product".to_string())),
            price: Set(price),
            quantity: Set(qty),
            subtotal: Set(subtotal),
            ..Default::default()
        }
    }).collect();
    order_items::Entity::insert_many(order_items_to_insert).exec(&ctx.db).await?;

    let worker_args = OrderWorkerArgs { order_id };
    OrderCreationWorker::perform_later(&ctx, worker_args).await?;

    cart_items::Entity::delete_many()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .exec(&ctx.db)
        .await?;
    carts::Entity::delete_by_id(cart_uuid)
        .exec(&ctx.db)
        .await?;

    let jar = jar.remove(Cookie::new(cookie_name, ""));

    Ok((
        jar,
        Json(CheckoutResponse {
            success: true,
            order_name: None,
            invoice_name: None,
            total: Some(total_f64),
            error: None,
        }),
    ))
}

pub async fn order_success(
    ViewEngine(v): ViewEngine<TeraView>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    let order_ref = params.get("ref").cloned().unwrap_or_default();
    let invoice_ref = params.get("inv").cloned().unwrap_or_default();
    let total = params.get("total").cloned().unwrap_or_else(|| "0.00".to_string());

    format::render().view(
        &v,
        "shop/order_success.html",
        &serde_json::json!({
            "order_ref": order_ref,
            "invoice_ref": invoice_ref,
            "total": total,
            "current_user": user,
        }),
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/checkout", get(checkout_page))
        .add("/api/checkout", post(submit_checkout))
        .add("/order/success", get(order_success))
}
