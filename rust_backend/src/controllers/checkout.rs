#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use crate::models::_entities::{cart_items, carts, configs, products, users};
use axum::extract::Query;
use axum::http::HeaderMap;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CartItemRender {
    pub id: i32,
    pub name: String,
    pub price: f64,
    pub quantity: i32,
    pub subtotal: f64,
    pub image_filename: Option<String>,
}

#[derive(Deserialize)]
pub struct CustomerInfo {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub zip: Option<String>,
}

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub customer: CustomerInfo,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub success: bool,
    pub order_name: Option<String>,
    pub invoice_name: Option<String>,
    pub error: Option<String>,
}

pub async fn get_current_user(
    ctx: &AppContext,
    cookie_header: Option<String>,
) -> Option<users::Model> {
    let cookie_str = cookie_header?;
    let token = cookie_str
        .split(';')
        .find(|s| s.trim().starts_with("token="))?
        .split('=')
        .nth(1)?;
    let jwt_config = ctx.config.get_jwt_config().ok()?;
    let auth = loco_rs::auth::jwt::JWT::new(&jwt_config.secret)
        .validate(token)
        .ok()?;
    users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await
        .ok()
}

pub async fn checkout_page(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    ViewEngine(v): ViewEngine<TeraView>,
    headers: HeaderMap,
) -> Result<Response> {
    let cookie_name = "rsv_cart_session";
    let mut render_items = Vec::new();
    let mut grand_total = 0.0;
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    if let Some(cookie) = jar.get(cookie_name) {
        if let Ok(cart_uuid) = Uuid::parse_str(cookie.value()) {
            let items = cart_items::Entity::find()
                .filter(cart_items::Column::CartId.eq(cart_uuid))
                .all(&ctx.db)
                .await?;

            if !items.is_empty() {
                let mut product_ids = Vec::new();
                let mut item_quantities = std::collections::HashMap::new();

                for item in items {
                    let hex_str = item.product_id.simple().to_string();
                    if let Ok(prod_id) = i32::from_str_radix(&hex_str, 16) {
                        product_ids.push(prod_id);
                        item_quantities.insert(prod_id, item.quantity);
                    }
                }

                let db_products = products::Entity::find()
                    .filter(products::Column::Id.is_in(product_ids))
                    .all(&ctx.db)
                    .await?;

                for prod in db_products {
                    let qty = *item_quantities.get(&prod.id).unwrap_or(&1);
                    let price_f64 = prod
                        .price
                        .map(|p| p.to_string().parse::<f64>().unwrap_or(0.0))
                        .unwrap_or(0.0);
                    let subtotal = price_f64 * (qty as f64);
                    grand_total += subtotal;

                    render_items.push(CartItemRender {
                        id: prod.id,
                        name: prod.name.unwrap_or_else(|| {
                            "Producto sin nombre".to_string()
                        }),
                        price: price_f64,
                        quantity: qty,
                        subtotal,
                        image_filename: prod.image_filename,
                    });
                }
            }
        }
    }

    format::render().view(
        &v,
        "shop/checkout.html",
        &serde_json::json!({
            "items": render_items,
            "total": grand_total,
            "current_user": user,
        }),
    )
}

pub async fn submit_checkout(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    Json(params): Json<CheckoutRequest>,
) -> Result<(CookieJar, Json<CheckoutResponse>)> {
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
                    error: Some("Carrito no encontrado".to_string()),
                }),
            ));
        }
    };

    let items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .all(&ctx.db)
        .await
        .map_err(|e| Error::string(&e.to_string()))?;

    if items.is_empty() {
        return Ok((
            jar,
            Json(CheckoutResponse {
                success: false,
                order_name: None,
                invoice_name: None,
                error: Some("El carrito está vacío".to_string()),
            }),
        ));
    }

    let mut product_ids = Vec::new();
    let mut item_map = std::collections::HashMap::new();
    for item in &items {
        let hex_str = item.product_id.simple().to_string();
        if let Ok(prod_id) = i32::from_str_radix(&hex_str, 16) {
            product_ids.push(prod_id);
            item_map.insert(prod_id, item.quantity);
        }
    }

    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(product_ids))
        .all(&ctx.db)
        .await
        .map_err(|e| Error::string(&e.to_string()))?;

    let mut odoo_items = Vec::new();
    for prod in db_products {
        let qty = *item_map.get(&prod.id).unwrap_or(&1);
        let price_f64 = prod
            .price
            .map(|p| p.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
        odoo_items.push(serde_json::json!({
            "product_id": prod.id,
            "name": prod.name.unwrap_or_else(|| "Product".to_string()),
            "price": price_f64,
            "quantity": qty,
        }));
    }

    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq("webhook_token"))
        .one(&ctx.db)
        .await
        .map_err(|e| Error::string(&e.to_string()))?;

    let token = config.and_then(|c| c.value).unwrap_or_default();

    let odoo_domain = configs::Entity::find()
        .filter(configs::Column::Key.eq("odoo_base_url"))
        .one(&ctx.db)
        .await?
        .and_then(|c| c.value)
        .unwrap_or_else(|| "http://localhost:8072".to_string());

    let odoo_url = format!("{}/api/orders/create", odoo_domain);

    let payload = serde_json::json!({
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
                return Ok((
                    jar,
                    Json(CheckoutResponse {
                        success: false,
                        order_name: None,
                        invoice_name: None,
                        error: Some(format!(
                            "Odoo respondió con error {}: {}",
                            status, body
                        )),
                    }),
                ));
            }

            let odoo_resp: serde_json::Value = resp.json().await.unwrap_or_default();

            if odoo_resp.get("error").is_some() {
                return Ok((
                    jar,
                    Json(CheckoutResponse {
                        success: false,
                        order_name: None,
                        invoice_name: None,
                        error: Some(odoo_resp["error"]
                            .as_str()
                            .unwrap_or("Error desconocido de Odoo")
                            .to_string()),
                    }),
                ));
            }

            cart_items::Entity::delete_many()
                .filter(cart_items::Column::CartId.eq(cart_uuid))
                .exec(&ctx.db)
                .await
                .map_err(|e| Error::string(&e.to_string()))?;

            carts::Entity::delete_by_id(cart_uuid)
                .exec(&ctx.db)
                .await
                .map_err(|e| Error::string(&e.to_string()))?;

            let jar = jar.remove(Cookie::new(cookie_name, ""));

            let order_name = odoo_resp["order_name"]
                .as_str()
                .map(|s| s.to_string());
            let invoice_name = odoo_resp["invoice_name"]
                .as_str()
                .map(|s| s.to_string());

            Ok((
                jar,
                Json(CheckoutResponse {
                    success: true,
                    order_name,
                    invoice_name,
                    error: None,
                }),
            ))
        }
        Err(e) => Ok((
            jar,
            Json(CheckoutResponse {
                success: false,
                order_name: None,
                invoice_name: None,
                error: Some(format!("Error de conexión con Odoo: {}", e)),
            }),
        )),
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
