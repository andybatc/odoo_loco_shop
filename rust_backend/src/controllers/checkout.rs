#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use crate::controllers::views::get_current_user;
use crate::models::_entities::{cart_items, carts, configs, order_items, products};
use crate::models::_entities::orders as orders_entity;
use crate::models::_entities::payment_methods as payment_methods_entity;
use crate::models::cart_helpers;
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
    jar: CookieJar,
    ViewEngine(v): ViewEngine<TeraView>,
    headers: HeaderMap,
) -> Result<Response> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    let (items, total) = if let Some(cookie) = jar.get("rsv_cart_session") {
        if let Ok(cart_uuid) = Uuid::parse_str(cookie.value()) {
            let cart = cart_helpers::load_cart(&ctx, cart_uuid).await?;
            (cart.items, cart.total)
        } else {
            (vec![], 0.0)
        }
    } else {
        (vec![], 0.0)
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

    let cart_uuid = match jar.get(cookie_name) {
        Some(cookie) => match Uuid::parse_str(cookie.value()) {
            Ok(id) => id,
            Err(_) => {
                return Ok((
                    jar,
                    Json(CheckoutResponse {
                        success: false,
                        order_name: None,
                        invoice_name: None,
                        total: None,
                        error: Some("Carrito no encontrado".to_string()),
                    }),
                ));
            }
        },
        None => {
            return Ok((
                jar,
                Json(CheckoutResponse {
                    success: false,
                    order_name: None,
                    invoice_name: None,
                    total: None,
                    error: Some("Carrito no encontrado".to_string()),
                }),
            ));
        }
    };

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
    let mut odoo_items = Vec::new();
    for prod in &db_products {
        let qty = *item_map.get(&prod.id).unwrap_or(&1);
        let price = prod.price.unwrap_or(sea_orm::prelude::Decimal::ZERO);
        let price_f64 = price.to_string().parse::<f64>().unwrap_or(0.0);
        let subtotal = price * sea_orm::prelude::Decimal::from(qty as i64);
        total += subtotal;
        odoo_items.push(serde_json::json!({
            "product_id": prod.id,
            "name": prod.name.clone().unwrap_or_else(|| "Product".to_string()),
            "price": price_f64,
            "quantity": qty,
        }));
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

    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq("webhook_token"))
        .one(&ctx.db)
        .await
        ?;

    let token = config.and_then(|c| c.value).unwrap_or_default();

    let odoo_domain = configs::Entity::find()
        .filter(configs::Column::Key.eq("odoo_base_url"))
        .one(&ctx.db)
        .await?
        .and_then(|c| c.value)
        .unwrap_or_else(|| "http://localhost:8072".to_string());

    let odoo_url = format!("{}/api/orders/create", odoo_domain);

    let mut payload = serde_json::json!({
        "customer": {
            "name": params.customer.name,
            "email": params.customer.email,
            "phone": params.customer.phone,
            "street": params.customer.street,
            "city": params.customer.city,
            "zip": params.customer.zip,
        },
        "items": odoo_items,
    });

    if let Some(pm_id) = params.payment_method_id {
        payload["payment_method_id"] = serde_json::json!(pm_id);
    }

    let client = reqwest::Client::new();
    let response = client
        .post(odoo_url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                orders_entity::ActiveModel {
                    id: Set(order_id),
                    status: Set("failed".to_string()),
                    ..Default::default()
                }.update(&ctx.db).await?;
                return Ok((
                    jar,
                    Json(CheckoutResponse {
                        success: false,
                        order_name: None,
                        invoice_name: None,
                        total: Some(total_f64),
                        error: Some(format!(
                            "Odoo respondió con error {}: {}",
                            status, body
                        )),
                    }),
                ));
            }

            let odoo_resp: serde_json::Value = resp.json().await.unwrap_or_default();

            if odoo_resp.get("error").is_some() {
                orders_entity::ActiveModel {
                    id: Set(order_id),
                    status: Set("failed".to_string()),
                    ..Default::default()
                }.update(&ctx.db).await?;
                return Ok((
                    jar,
                    Json(CheckoutResponse {
                        success: false,
                        order_name: None,
                        invoice_name: None,
                        total: Some(total_f64),
                        error: Some(odoo_resp["error"]
                            .as_str()
                            .unwrap_or("Error desconocido de Odoo")
                            .to_string()),
                    }),
                ));
            }

            let order_name = odoo_resp["order_name"]
                .as_str()
                .map(|s| s.to_string());
            let invoice_name = odoo_resp["invoice_name"]
                .as_str()
                .map(|s| s.to_string());

            let confirmed_model = orders_entity::ActiveModel {
                id: Set(order_id),
                status: Set("confirmed".to_string()),
                odoo_order_name: Set(order_name.clone()),
                odoo_invoice_name: Set(invoice_name.clone()),
                ..Default::default()
            }.update(&ctx.db).await?;

            let _ = crate::mailers::order::OrderMailer::send_confirmation(&ctx, &confirmed_model).await;

            cart_items::Entity::delete_many()
                .filter(cart_items::Column::CartId.eq(cart_uuid))
                .exec(&ctx.db)
                .await
                ?;

            carts::Entity::delete_by_id(cart_uuid)
                .exec(&ctx.db)
                .await
                ?;

            let jar = jar.remove(Cookie::new(cookie_name, ""));

            Ok((
                jar,
                Json(CheckoutResponse {
                    success: true,
                    order_name,
                    invoice_name,
                    total: Some(total_f64),
                    error: None,
                }),
            ))
        }
        Err(e) => {
            orders_entity::ActiveModel {
                id: Set(order_id),
                status: Set("failed".to_string()),
                ..Default::default()
            }.update(&ctx.db).await?;
            Ok((
                jar,
                Json(CheckoutResponse {
                    success: false,
                    order_name: None,
                    invoice_name: None,
                    total: Some(total_f64),
                    error: Some(format!("Error de conexión con Odoo: {}", e)),
                }),
            ))
        }
    }
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
