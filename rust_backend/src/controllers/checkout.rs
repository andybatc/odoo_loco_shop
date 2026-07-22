#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use crate::controllers::views::get_current_user;
use crate::models::_entities::{cart_items, carts, order_items, products, shipping_rates, users};
use crate::models::_entities::orders as orders_entity;
use crate::models::_entities::payment_methods as payment_methods_entity;
use crate::models::cart_helpers;
use crate::models::config_cache;
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
use std::collections::{BTreeMap, HashMap};
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
    pub country: Option<String>,
    pub state: Option<String>,
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

/// Fetch countries + states directly from Odoo DB (mirrors Odoo's own dropdowns).
pub async fn get_countries_with_states(ctx: &AppContext) -> BTreeMap<String, Vec<String>> {
    // ponytail: cache in Redis for 10min so we don't hit Odoo DB on every checkout load
    let cache_key = "checkout:countries";
    if let Ok(Some(cached)) = ctx.cache.get::<BTreeMap<String, Vec<String>>>(cache_key).await {
        return cached;
    }

    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let odoo_uri = crate::models::_entities::configs::Entity::find()
        .filter(crate::models::_entities::configs::Column::Key.eq("odoo_db_uri"))
        .one(&ctx.db)
        .await
        .ok()
        .flatten()
        .and_then(|c| c.value)
        .unwrap_or_else(|| "postgres://odoo:postgres@localhost:5432/odoo_prod".to_string());

    if let Ok(odoo_db) = sea_orm::Database::connect(&odoo_uri).await {
        let backend = odoo_db.get_database_backend();
        if let Ok(rows) = odoo_db
            .query_all(sea_orm::Statement::from_string(
                backend,
                "SELECT rc.name->>'en_US' AS country, rcs.name AS state
                 FROM res_country rc
                 JOIN res_country_state rcs ON rcs.country_id = rc.id
                 ORDER BY rc.name->>'en_US', rcs.name".to_string(),
            ))
            .await
        {
            for row in &rows {
                let country: Option<String> = row.try_get_by_index(0).ok();
                let state: Option<String> = row.try_get_by_index(1).ok();
                if let (Some(c), Some(s)) = (country, state) {
                    map.entry(c).or_default().push(s);
                }
            }
        }
    }

    let _ = ctx.cache.insert_with_expiry(cache_key, &map, std::time::Duration::from_secs(600)).await;
    map
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

    let user_data = user.as_ref().map(|u| serde_json::json!({
        "name": u.name,
        "email": u.email,
        "phone": u.phone,
        "street": u.street,
        "city": u.city,
        "zip": u.zip,
        "country": u.country,
        "state": u.state,
    }));
    tracing::debug!("checkout_page: user={:?}, user_data={:?}", user.as_ref().map(|u| u.email.as_str()), user_data);

    // ponytail: pass rates + countries for client-side estimation and dropdowns
    let shipping_rates_list = shipping_rates::Entity::find().all(&ctx.db).await.unwrap_or_default();
    let shipping_rates_json = serde_json::to_value(&shipping_rates_list).unwrap_or_default();

    let countries = get_countries_with_states(&ctx).await;

    format::render().view(
        &v,
        "shop/checkout.html",
        serde_json::json!({
            "items": items,
            "total": total,
            "current_user": user,
            "user_data": user_data,
            "payment_methods": payment_methods,
            "shipping_rates": shipping_rates_json,
            "countries": countries,
        }),
    )
}

pub(crate) async fn calc_shipping(
    db: &DatabaseConnection,
    items: &[products::Model],
    country: &str,
    state: &str,
) -> Result<(sea_orm::prelude::Decimal, String), sea_orm::DbErr> {
    // ponytail: consolidate by highest rate — safest assumption for furthest product
    let mut origins: Vec<(&str, &str)> = Vec::new();
    for product in items {
        if let (Some(c), Some(s)) = (&product.warehouse_country, &product.warehouse_state) {
            if !origins.iter().any(|(oc, os)| oc == c && os == s) {
                origins.push((c, s));
            }
        }
    }

    if origins.is_empty() {
        return Ok((sea_orm::prelude::Decimal::ZERO, "Sin origen definido".to_string()));
    }

    let mut max_rate = sea_orm::prelude::Decimal::ZERO;
    let mut origin_desc = String::new();

    for (oc, os) in &origins {
        let rate = match crate::models::shipping_rates::find_rate(db, oc, os, country, state).await? {
            Some(r) => r,
            None => crate::models::shipping_rates::find_rate_by_country(db, oc, country, state)
                .await?
                .unwrap_or(sea_orm::prelude::Decimal::ZERO),
        };

        if rate > max_rate {
            max_rate = rate;
            origin_desc = format!("{}, {}", os, oc);
        }
    }

    Ok((max_rate, origin_desc))
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

    let dest_country = params.customer.country.as_deref().unwrap_or("");
    let dest_state = params.customer.state.as_deref().unwrap_or("");

    let (shipping_cost, _shipping_origin) = calc_shipping(&ctx.db, &db_products, dest_country, dest_state).await?;

    // ponytail: local delivery override — all products ship from same country/state as destination
    let all_local = db_products.iter().all(|p| {
        p.warehouse_country.as_deref() == Some(dest_country)
            && p.warehouse_state.as_deref() == Some(dest_state)
    });
    let shipping_cost = if all_local && !dest_country.is_empty() {
        let local_rate = crate::models::config_cache::get_cached_config(&ctx, "shipping_local_rate")
            .await
            .unwrap_or(None)
            .and_then(|v| v.parse::<f64>().ok())
            .map(sea_orm::prelude::Decimal::try_from)
            .and_then(Result::ok)
            .unwrap_or(sea_orm::prelude::Decimal::ZERO);
        if local_rate > sea_orm::prelude::Decimal::ZERO { local_rate } else { shipping_cost }
    } else {
        shipping_cost
    };

    total += shipping_cost;

    let total_f64 = total.to_string().parse::<f64>().unwrap_or(0.0);

    let checkout_user = get_current_user(&ctx, headers.get("cookie").and_then(|h| h.to_str().ok()).map(|s| s.to_string())).await;

    let order_id = Uuid::new_v4();
    let order = orders_entity::ActiveModel {
        id: Set(order_id),
        user_id: Set(checkout_user.as_ref().map(|u| u.id)),
        customer_name: Set(params.customer.name.clone()),
        customer_email: Set(params.customer.email.clone()),
        customer_phone: Set(params.customer.phone.clone()),
        customer_street: Set(params.customer.street.clone()),
        customer_city: Set(params.customer.city.clone()),
        customer_zip: Set(params.customer.zip.clone()),
        customer_country: Set(params.customer.country.clone()),
        customer_state: Set(params.customer.state.clone()),
        shipping_cost: Set(Some(shipping_cost)),
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

    // ponytail: save checkout data to user profile if logged in
    if let Some(ref user) = checkout_user {
        use sea_orm::ActiveValue::Set;
        let mut active: users::ActiveModel = user.clone().into();
        if let Some(v) = &params.customer.phone { if !v.is_empty() { active.phone = Set(Some(v.clone())); } }
        if let Some(v) = &params.customer.street { if !v.is_empty() { active.street = Set(Some(v.clone())); } }
        if let Some(v) = &params.customer.city { if !v.is_empty() { active.city = Set(Some(v.clone())); } }
        if let Some(v) = &params.customer.zip { if !v.is_empty() { active.zip = Set(Some(v.clone())); } }
        if let Some(v) = &params.customer.country { if !v.is_empty() { active.country = Set(Some(v.clone())); } }
        if let Some(v) = &params.customer.state { if !v.is_empty() { active.state = Set(Some(v.clone())); } }
        active.update(&ctx.db).await.ok();
    }

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

#[debug_handler]
pub(crate) async fn create_stripe_session(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(params): Json<CheckoutRequest>,
) -> Result<Json<StripeSessionResponse>> {
    // Validate email
    let email_re = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    if !email_re.is_match(&params.customer.email) {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("Email inválido".to_string()),
        }));
    }
    if params.customer.name.trim().is_empty() {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("El nombre es obligatorio".to_string()),
        }));
    }

    // Get cart (same as submit_checkout)
    let cart_uuid = {
        let user = get_current_user(&ctx, headers.get("cookie").and_then(|h| h.to_str().ok()).map(|s| s.to_string())).await;
        if let Some(ref u) = user {
            let cart = carts::Entity::find()
                .filter(carts::Column::UserId.eq(u.pid))
                .one(&ctx.db).await?;
            match cart {
                Some(c) => c.id,
                None => return Ok(Json(StripeSessionResponse {
                    success: false, url: None, error: Some("Carrito no encontrado".to_string()),
                })),
            }
        } else {
            let cookie_val = headers.get("cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(';').find(|p| p.trim().starts_with("rsv_cart_session=")))
                .and_then(|s| s.split('=').nth(1).map(|v| v.trim().to_string()));
            match cookie_val.and_then(|v| Uuid::parse_str(&v).ok()) {
                Some(id) => id,
                None => return Ok(Json(StripeSessionResponse {
                    success: false, url: None, error: Some("Carrito no encontrado".to_string()),
                })),
            }
        }
    };

    // Load cart items
    let items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .all(&ctx.db).await?;
    if items.is_empty() {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("El carrito está vacío".to_string()),
        }));
    }

    let mut product_ids = Vec::new();
    let mut item_map = HashMap::new();
    for item in &items {
        product_ids.push(item.product_id);
        item_map.insert(item.product_id, item.quantity);
    }

    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(product_ids))
        .all(&ctx.db).await?;

    // Calculate shipping with local delivery override (same logic as submit_checkout)
    let dest_country = params.customer.country.as_deref().unwrap_or("");
    let dest_state = params.customer.state.as_deref().unwrap_or("");
    let (mut shipping_cost, _) = calc_shipping(&ctx.db, &db_products, dest_country, dest_state).await?;
    let all_local = db_products.iter().all(|p| {
        p.warehouse_country.as_deref() == Some(dest_country)
            && p.warehouse_state.as_deref() == Some(dest_state)
    });
    if all_local && !dest_country.is_empty() {
        let local_rate = crate::models::config_cache::get_cached_config(&ctx, "shipping_local_rate")
            .await.unwrap_or(None)
            .and_then(|v| v.parse::<f64>().ok())
            .map(sea_orm::prelude::Decimal::try_from)
            .and_then(Result::ok)
            .unwrap_or(sea_orm::prelude::Decimal::ZERO);
        if local_rate > sea_orm::prelude::Decimal::ZERO { shipping_cost = local_rate; }
    }

    // Build Stripe line items
    let mut stripe_line_items: Vec<serde_json::Value> = Vec::new();
    for prod in &db_products {
        let qty = *item_map.get(&prod.id).unwrap_or(&1) as u64;
        let price_cents = prod.price
            .map(|p| (p.to_string().parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64)
            .unwrap_or(0);
        let name = prod.name.clone().unwrap_or_else(|| "Producto".to_string());

        stripe_line_items.push(serde_json::json!({
            "quantity": qty,
            "price_data": {
                "currency": "mxn",
                "product_data": { "name": name },
                "unit_amount": price_cents,
            }
        }));
    }

    // Add shipping as a Stripe line item
    if shipping_cost > sea_orm::prelude::Decimal::ZERO {
        let shipping_cents = shipping_cost.to_string().parse::<f64>().unwrap_or(0.0) * 100.0;
        stripe_line_items.push(serde_json::json!({
            "quantity": 1,
            "price_data": {
                "currency": "mxn",
                "product_data": { "name": "Envío" },
                "unit_amount": shipping_cents.round() as i64,
            }
        }));
    }

    // Get Stripe secret key
    let secret_key = crate::models::config_cache::get_cached_config(&ctx, "stripe_secret_key")
        .await?.unwrap_or_default();
    if secret_key.is_empty() || secret_key == "No configurado" {
        return Ok(Json(StripeSessionResponse {
            success: false, url: None, error: Some("Stripe no configurado".to_string()),
        }));
    }

    let base_url = crate::models::config_cache::get_cached_config(&ctx, "odoo_base_url")
        .await?.unwrap_or_else(|| "http://localhost:5150".to_string());

    let session_params = serde_json::json!({
        "mode": "payment",
        "success_url": format!("{}/order/success?session_id={{CHECKOUT_SESSION_ID}}", base_url),
        "cancel_url": format!("{}/checkout", base_url),
        "line_items": stripe_line_items,
        "customer_email": params.customer.email,
        "metadata": {
            "cart_uuid": cart_uuid.to_string(),
            "payment_method_id": params.payment_method_id.unwrap_or(0).to_string(),
        },
    });

    let http_client = reqwest::Client::new();
    let resp = http_client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .header("Authorization", format!("Bearer {}", secret_key))
        .json(&session_params)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Stripe HTTP error: {:?}", e);
            Error::string("Error de conexión con Stripe")
        })?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("Stripe API error: {}", body);
        return Err(Error::string("Error al crear sesión de pago en Stripe"));
    }

    let session: serde_json::Value = resp.json().await.map_err(|e| {
        tracing::error!("Stripe response parse error: {:?}", e);
        Error::string("Error al procesar respuesta de Stripe")
    })?;

    let session_url = session["url"].as_str().ok_or_else(|| {
        tracing::error!("Stripe session created without URL: {:?}", session);
        Error::string("Error al crear sesión de pago")
    })?.to_string();

    let session_id = session["id"].as_str().unwrap_or("");
    if session_id.is_empty() {
        return Err(Error::string("Error: sesión Stripe sin ID"));
    }

    // Store checkout data in Redis for the callback
    let checkout_data = serde_json::json!({
        "cart_uuid": cart_uuid,
        "customer": {
            "name": params.customer.name,
            "email": params.customer.email,
            "phone": params.customer.phone,
            "street": params.customer.street,
            "city": params.customer.city,
            "zip": params.customer.zip,
            "country": params.customer.country,
            "state": params.customer.state,
        },
        "payment_method_id": params.payment_method_id,
    });
    let _ = ctx.cache.insert_with_expiry(
        &format!("stripe:session:{}", session_id),
        &checkout_data,
        std::time::Duration::from_secs(3600),
    ).await;

    // Store shipping cost in Redis for the callback
    let _ = ctx.cache.insert_with_expiry(
        &format!("stripe:shipping:{}", session_id),
        &serde_json::json!({ "cost": shipping_cost.to_string() }),
        std::time::Duration::from_secs(3600),
    ).await;

    Ok(Json(StripeSessionResponse {
        success: true,
        url: Some(session_url),
        error: None,
    }))
}

/// Process a confirmed paid Stripe Checkout session.
/// Returns (order_id, total_f64, already_processed).
/// Idempotent: safe to call multiple times for the same session.
/// Shared between redirect callback and webhook handler.
pub(crate) async fn process_paid_session(
    ctx: &AppContext,
    session_id: &str,
    user: Option<users::Model>,
) -> Result<(Uuid, f64, bool)> {
    let secret_key = config_cache::get_cached_config(ctx, "stripe_secret_key")
        .await?.unwrap_or_default();
    if secret_key.is_empty() || secret_key == "No configurado" {
        return Err(Error::string("Stripe no configurado"));
    }

    let http_client = reqwest::Client::new();
    let resp = http_client
        .get(format!("https://api.stripe.com/v1/checkout/sessions/{}", session_id))
        .header("Authorization", format!("Bearer {}", secret_key))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Stripe HTTP error: {:?}", e);
            Error::string("Error de conexión con Stripe")
        })?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("Stripe retrieve error: {}", body);
        return Err(Error::BadRequest("Error al verificar el pago en Stripe".to_string()));
    }

    let session: serde_json::Value = resp.json().await.map_err(|e| {
        tracing::error!("Stripe response parse error: {:?}", e);
        Error::string("Error al procesar respuesta de Stripe")
    })?;

    if session["payment_status"].as_str() != Some("paid") {
        return Err(Error::BadRequest("El pago no fue completado".to_string()));
    }

    // Retrieve checkout data from Redis
    let redis_key = format!("stripe:session:{}", session_id);
    let checkout_data: Option<serde_json::Value> = ctx.cache.get(&redis_key).await.ok().flatten();
    let checkout_data = match checkout_data {
        Some(d) => d,
        None => {
            tracing::warn!("Checkout data expired for session {}", session_id);
            return Err(Error::BadRequest(
                "Sesión expirada. Por favor, intente nuevamente.".to_string(),
            ));
        }
    };

    // Check if already processed via Redis marker (atomic)
    let order_key = format!("stripe:order:{}", session_id);
    if let Ok(Some(_)) = ctx.cache.get::<String>(&order_key).await {
        tracing::info!("Stripe session {} already processed, skipping", session_id);
        return Ok((Uuid::default(), 0.0, true));
    }

    let cart_uuid_str = checkout_data["cart_uuid"].as_str().unwrap_or("");
    let cart_uuid = Uuid::parse_str(cart_uuid_str)
        .map_err(|_| Error::BadRequest("Datos de sesión inválidos".to_string()))?;

    // Secondary check: cart already cleared (fallback if Redis marker expired)
    let product_items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .all(&ctx.db)
        .await?;

    if product_items.is_empty() {
        tracing::warn!(
            "Cart already cleared for session {}, treating as already processed",
            session_id
        );
        return Ok((Uuid::default(), 0.0, true));
    }

    // Rebuild order data from stored checkout info
    let customer = &checkout_data["customer"];
    let mut pids = Vec::new();
    let mut item_qty = std::collections::HashMap::new();
    for item in &product_items {
        pids.push(item.product_id);
        item_qty.insert(item.product_id, item.quantity);
    }

    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(pids))
        .all(&ctx.db)
        .await?;

    // Recalculate total
    let shipping_key = format!("stripe:shipping:{}", session_id);
    let shipping_data: Option<serde_json::Value> =
        ctx.cache.get(&shipping_key).await.ok().flatten();
    let shipping_cost: sea_orm::prelude::Decimal = shipping_data
        .and_then(|v| v["cost"].as_str()?.parse::<f64>().ok())
        .map(|v| sea_orm::prelude::Decimal::try_from(v).unwrap_or_default())
        .unwrap_or_default();

    let mut total = shipping_cost;
    for prod in &db_products {
        let qty = *item_qty.get(&prod.id).unwrap_or(&1);
        let price = prod.price.unwrap_or_default();
        total += price * sea_orm::prelude::Decimal::from(qty as i64);
    }
    let total_f64 = total.to_string().parse::<f64>().unwrap_or(0.0);

    // Verify Stripe-collected amount matches expected
    let expected_cents = (total_f64 * 100.0).round() as i64;
    if let Some(actual_cents) = session["amount_total"].as_i64() {
        if actual_cents != expected_cents {
            tracing::error!(
                "Stripe amount mismatch: expected {} cents, got {} (session: {})",
                expected_cents,
                actual_cents,
                session_id
            );
            return Err(Error::BadRequest(
                "Monto del pago no coincide con el esperado".to_string(),
            ));
        }
    }

    // Create order
    let order_id = Uuid::new_v4();
    let order = orders_entity::ActiveModel {
        id: Set(order_id),
        user_id: Set(user.as_ref().map(|u| u.id)),
        customer_name: Set(customer["name"].as_str().unwrap_or("").to_string()),
        customer_email: Set(customer["email"].as_str().unwrap_or("").to_string()),
        customer_phone: Set(customer["phone"].as_str().map(|s| s.to_string())),
        customer_street: Set(customer["street"].as_str().map(|s| s.to_string())),
        customer_city: Set(customer["city"].as_str().map(|s| s.to_string())),
        customer_zip: Set(customer["zip"].as_str().map(|s| s.to_string())),
        customer_country: Set(customer["country"].as_str().map(|s| s.to_string())),
        customer_state: Set(customer["state"].as_str().map(|s| s.to_string())),
        shipping_cost: Set(Some(shipping_cost)),
        total: Set(total),
        status: Set("paid".to_string()),
        ..Default::default()
    };
    order.insert(&ctx.db).await?;

    // Mark as processed in Redis (atomic idempotency guard)
    let _ = ctx
        .cache
        .insert_with_expiry(
            &format!("stripe:order:{}", session_id),
            &order_id.to_string(),
            std::time::Duration::from_secs(3600),
        )
        .await;

    // Create order items
    let order_items_to_insert: Vec<order_items::ActiveModel> = db_products
        .iter()
        .map(|prod| {
            let qty = *item_qty.get(&prod.id).unwrap_or(&1);
            let price = prod.price.unwrap_or_default();
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
        })
        .collect();
    order_items::Entity::insert_many(order_items_to_insert)
        .exec(&ctx.db)
        .await?;

    // Dispatch Odoo sync
    let worker_args = crate::workers::order_creation::OrderWorkerArgs { order_id };
    crate::workers::order_creation::OrderCreationWorker::perform_later(ctx, worker_args)
        .await?;

    // Clear cart
    cart_items::Entity::delete_many()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .exec(&ctx.db)
        .await?;
    carts::Entity::delete_by_id(cart_uuid).exec(&ctx.db).await?;

    // Clean up Redis
    let _ = ctx.cache.remove(&redis_key).await;
    let _ = ctx.cache.remove(&shipping_key).await;

    tracing::info!("Order {} created from Stripe session {}", order_id, session_id);

    Ok((order_id, total_f64, false))
}

pub async fn order_success(
    ViewEngine(v): ViewEngine<TeraView>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    jar: CookieJar,
) -> Result<(CookieJar, Response)> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    // Stripe callback flow
    if let Some(session_id) = params.get("session_id") {
        let (order_id, total_f64, already_processed) =
            process_paid_session(&ctx, session_id, user.clone()).await?;

        let jar = if already_processed {
            jar
        } else {
            jar.remove(Cookie::new("rsv_cart_session", ""))
        };

        return Ok((jar, format::render().view(
            &v,
            "shop/order_success.html",
            serde_json::json!({
                "order_ref": order_id.to_string(),
                "invoice_ref": "",
                "total": format!("{:.2}", total_f64),
                "current_user": user,
                "already_processed": already_processed,
            }),
        )?));
    }

    // Existing non-Stripe flow
    let order_ref = params.get("ref").cloned().unwrap_or_default();
    let invoice_ref = params.get("inv").cloned().unwrap_or_default();
    let total = params.get("total").cloned().unwrap_or_else(|| "0.00".to_string());

    let response = format::render().view(
        &v, "shop/order_success.html",
        serde_json::json!({
            "order_ref": order_ref,
            "invoice_ref": invoice_ref,
            "total": total,
            "current_user": user,
            "already_processed": false,
        }),
    )?;

    Ok((jar, response))
}

#[derive(Serialize)]
pub struct StripeSessionResponse {
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/checkout", get(checkout_page))
        .add("/api/checkout", post(submit_checkout))
        .add("/api/checkout/stripe-session", post(create_stripe_session))
        .add("/order/success", get(order_success))
}
