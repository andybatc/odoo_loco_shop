#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::controllers::auth as auth_controller;
use crate::models::_entities::{users, configs, carts, cart_items, orders as orders_entity};
use crate::models::cart_helpers;
use crate::models::config_cache;
use crate::models::users::LoginParams;
use crate::views::auth::LoginResponse;
use axum::extract::Form;
use axum::http::HeaderMap;
use loco_rs::auth::jwt::JWT;
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use loco_rs::prelude::Json;
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::QueryFilter;
use serde::{Deserialize, Serialize};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use regex::Regex;
use sea_orm::QueryOrder;
use uuid::Uuid;

#[derive(Serialize)]
pub struct BaseContext {
    pub current_user: Option<users::Model>,
}

pub async fn get_current_user(ctx: &AppContext, cookie_header: Option<String>) -> Option<users::Model> {
    let cookie_str = cookie_header?;

    // 1. Extraer el token
    let token = cookie_str
        .split(';')
        .find(|s| s.trim().starts_with("token="))?
        .split('=')
        .nth(1)?;

    // 2. Validar el JWT
    let jwt_config = ctx.config.get_jwt_config().ok()?;

    // CORRECCIÓN: Quitamos el ::<loco_rs::auth::jwt::UserClaims>
    let auth = JWT::new(&jwt_config.secret).validate(token).ok()?;

    // 3. Buscar usuario en DB
    users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await
        .ok()
}

pub async fn home_page(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    // Usamos 'include_utils!' o el helper de renderizado de Loco
    format::render().template(
        "home.html",
        serde_json::json!({
            "current_user": user
        }),
    )
}

async fn login_display(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    if let Ok(current_dir) = std::env::current_dir() {
        tracing::info!("Directorio actual de ejecución: {:?}", current_dir);
    }
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));

    // 2. Obtener el usuario (si existe)
    let user = get_current_user(&ctx, cookie_header).await;

    let html_path = "assets/views/auth/login.html";
    let html = std::fs::read_to_string(html_path).map_err(|e| {
        tracing::error!("Error leyendo el HTML ({}) : {:?}", html_path, e);
        Error::string("No se encuentra la plantilla de login")
    })?;

    format::render().template(
        &html,
        serde_json::json!({
            "current_user": user
        }),
    )
}

async fn login_web(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<LoginParams>,
) -> Result<Response> {
    // --- EL PUENTE ---
    let login_json = Json(params);
    let api_response = auth_controller::login(State(ctx.clone()), login_json).await?;

    let body_bytes = axum::body::to_bytes(api_response.into_body(), 1024 * 10)
        .await
        .map_err(|_| Error::string("Error al leer respuesta de autenticación"))?;

    let login_res: LoginResponse = serde_json::from_slice(&body_bytes)
        .map_err(|_| Error::string("Error al procesar respuesta de autenticación"))?;

    if let Ok(user_id) = Uuid::parse_str(&login_res.pid) {
        if let Some(guest_id) = read_guest_cart_cookie(&headers) {
            tracing::info!("login_web: merging guest cart {} into user {}", guest_id, user_id);
            merge_guest_cart_into_user(&ctx, guest_id, user_id).await?;
        }
    }

    let jwt_config = ctx.config.get_jwt_config()?;
    let token_cookie = format!(
        "token={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        login_res.token, jwt_config.expiration
    );
    let clear_guest_cookie = "rsv_cart_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Response::builder()
        .header("Set-Cookie", token_cookie)
        .header("Set-Cookie", clear_guest_cookie)
        .header("HX-Redirect", "/")
        .body(axum::body::Body::empty())
        .map_err(|_| Error::string("Error al generar respuesta de autenticación"))
}

fn read_guest_cart_cookie(headers: &HeaderMap) -> Option<Uuid> {
    let cookie_str = headers.get("cookie")?.to_str().ok()?;
    let found = cookie_str
        .split(';')
        .find(|s| s.trim().starts_with("rsv_cart_session="))?;
    let val = found.split('=').nth(1)?;
    Uuid::parse_str(val.trim()).ok()
}

async fn merge_guest_cart_into_user(
    ctx: &AppContext,
    guest_cart_id: Uuid,
    user_id: Uuid,
) -> std::result::Result<(), Error> {
    let guest_cart = match carts::Entity::find_by_id(guest_cart_id).one(&ctx.db).await? {
        Some(c) => c,
        None => return Ok(()),
    };

    let user_cart = carts::Entity::find()
        .filter(carts::Column::UserId.eq(user_id))
        .one(&ctx.db)
        .await?;

    match user_cart {
        Some(uc) => {
            let guest_items = cart_items::Entity::find()
                .filter(cart_items::Column::CartId.eq(guest_cart_id))
                .all(&ctx.db)
                .await?;

            for item in guest_items {
                let existing = cart_items::Entity::find()
                    .filter(cart_items::Column::CartId.eq(uc.id))
                    .filter(cart_items::Column::ProductId.eq(item.product_id))
                    .one(&ctx.db)
                    .await?;

                if let Some(e) = existing {
                    let qty = e.quantity;
                    let mut active: cart_items::ActiveModel = e.into();
                    active.quantity = Set(qty + item.quantity);
                    active.update(&ctx.db).await?;
                } else {
                    cart_items::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        cart_id: Set(uc.id),
                        product_id: Set(item.product_id),
                        quantity: Set(item.quantity),
                    }
                    .insert(&ctx.db)
                    .await?;
                }
            }

            carts::Entity::delete_by_id(guest_cart_id)
                .exec(&ctx.db)
                .await?;
            tracing::info!(
                "merge_guest_cart: merged guest {} into user cart {}, guest deleted",
                guest_cart_id,
                uc.id
            );
        }
        None => {
            let mut active: carts::ActiveModel = guest_cart.into();
            active.user_id = Set(Some(user_id));
            active.update(&ctx.db).await?;
            tracing::info!(
                "merge_guest_cart: assigned guest cart {} to user {}",
                guest_cart_id,
                user_id
            );
        }
    }

    Ok(())
}

async fn register_display() -> Result<Response> {
    let html = std::fs::read_to_string("assets/views/auth/register.html")
        .map_err(|_| Error::string("No se encuentra la plantilla de registro"))?;
    format::html(&html)
}

#[derive(Deserialize)]
pub struct ProfileForm {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub state: Option<String>,
}

pub async fn profile_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(CookieJar, Response)> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    if user.is_none() {
        let html = std::fs::read_to_string("assets/static/403.html")
            .map_err(|_| Error::string("Error al cargar la página de acceso denegado"))?;
        let response = Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .map_err(|_| Error::string("Error al generar respuesta"))?;
        return Ok((jar, response));
    }

    let response = format::render().view(
        &v,
        "auth/profile.html",
        serde_json::json!({
            "current_user": user,
        }),
    )?;

    Ok((jar, response))
}

#[debug_handler]
pub async fn update_profile(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<ProfileForm>,
) -> Result<(CookieJar, Response)> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    let Some(user) = user else {
        let html = std::fs::read_to_string("assets/static/403.html")
            .map_err(|_| Error::string("Error al cargar la página de acceso denegado"))?;
        let response = Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .map_err(|_| Error::string("Error al generar respuesta"))?;
        return Ok((jar, response));
    };

    use sea_orm::ActiveValue::Set;
    let mut active: users::ActiveModel = user.clone().into();
    active.name = Set(form.name);
    active.email = Set(form.email);
    active.phone = Set(form.phone);
    active.street = Set(form.street);
    active.city = Set(form.city);
    active.zip = Set(form.zip);
    active.country = Set(form.country);
    active.state = Set(form.state);
    active.update(&ctx.db).await.map_err(|e| {
        tracing::error!("Error actualizando perfil: {:?}", e);
        Error::string("Error al guardar los datos del perfil")
    })?;

    let updated_user = users::Entity::find_by_id(user.id)
        .one(&ctx.db)
        .await
        .map_err(|_| Error::string("Error al cargar perfil"))?;

    let response = format::render().view(
        &v,
        "auth/profile.html",
        serde_json::json!({
            "current_user": updated_user,
            "success": true,
        }),
    )?;

    Ok((jar, response))
}

#[derive(Deserialize)]
pub struct ConfigUpdateForm {
    pub token: Option<String>,
    pub odoo_url: Option<String>,
    pub shipping_default_rate: Option<String>,
    pub shipping_local_rate: Option<String>,
    pub stripe_secret_key: Option<String>,
}

pub async fn config_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(CookieJar, Response)> {
    let token_value = config_cache::get_cached_config(&ctx, "webhook_token")
        .await
        .map_err(|e| {
            tracing::error!("Error consultando cache: {:?}", e);
            Error::string("Error al conectar con la base de datos")
        })?
        .unwrap_or_else(|| "No configurado".to_string());

    let odoo_url_value = config_cache::get_cached_config(&ctx, "odoo_base_url")
        .await
        .map_err(|e| {
            tracing::error!("Error consultando cache: {:?}", e);
            Error::string("Error al conectar con la base de datos")
        })?
        .unwrap_or_else(|| "http://localhost:8072".to_string());

    let shipping_default_rate = config_cache::get_cached_config(&ctx, "shipping_default_rate")
        .await
        .map_err(|e| {
            tracing::error!("Error consultando cache: {:?}", e);
            Error::string("Error al conectar con la base de datos")
        })?
        .unwrap_or_else(|| "10.00".to_string());

    let shipping_local_rate = config_cache::get_cached_config(&ctx, "shipping_local_rate")
        .await
        .map_err(|e| {
            tracing::error!("Error consultando cache: {:?}", e);
            Error::string("Error al conectar con la base de datos")
        })?
        .unwrap_or_else(|| "0.00".to_string());

    let stripe_secret_key = config_cache::get_cached_config(&ctx, "stripe_secret_key")
        .await
        .map_err(|e| {
            tracing::error!("Error consultando cache: {:?}", e);
            Error::string("Error al conectar con la base de datos")
        })?
        .unwrap_or_else(|| "No configurado".to_string());

    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    if user.as_ref().is_none_or(|u| u.role != "admin") {
        let html = std::fs::read_to_string("assets/static/403.html")
            .map_err(|_| Error::string("Error al cargar la página de acceso denegado"))?;
        let response = Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .map_err(|_| Error::string("Error al generar respuesta"))?;
        return Ok((jar, response));
    }

    let csrf_token = uuid::Uuid::new_v4().to_string();
    let csrf_cookie = Cookie::build(("csrf_token", csrf_token.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build();
    let jar = jar.add(csrf_cookie);

    let response = format::render().view(
        &v,
        "config/ui.html",
        serde_json::json!({
            "current_user": user,
            "current_token": token_value,
            "odoo_base_url": odoo_url_value,
            "shipping_default_rate": shipping_default_rate,
            "shipping_local_rate": shipping_local_rate,
            "stripe_secret_key": stripe_secret_key,
            "csrf_token": csrf_token,
        }),
    )?;

    Ok((jar, response))
}

fn read_cookie_val(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_str = headers.get("cookie")?.to_str().ok()?;
    cookie_str.split(';').find(|s| s.trim().starts_with(name))?.split('=').nth(1).map(|s| s.trim().to_string())
}

pub async fn cart_display(
    State(ctx): State<AppContext>,
    ViewEngine(v): ViewEngine<TeraView>,
    headers: HeaderMap,
) -> Result<Response> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    let (items, total) = if let Some(ref u) = user {
        let cart = carts::Entity::find()
            .filter(carts::Column::UserId.eq(u.pid))
            .one(&ctx.db)
            .await?;
        match cart {
            Some(c) => {
                let loaded = cart_helpers::load_cart(&ctx, c.id).await?;
                tracing::info!("🛒 Cart display logged-in: {} items, total={}", loaded.items.len(), loaded.total);
                (loaded.items, loaded.total)
            }
            None => (vec![], 0.0),
        }
    } else if let Some(ref val) = read_cookie_val(&headers, "rsv_cart_session") {
        match Uuid::parse_str(val) {
            Ok(cart_uuid) => {
                let loaded = cart_helpers::load_cart(&ctx, cart_uuid).await?;
                tracing::info!("🛒 Cart display guest: {} items, total={}", loaded.items.len(), loaded.total);
                (loaded.items, loaded.total)
            }
            Err(_) => {
                tracing::warn!("🛒 Invalid cart UUID in cookie: {}", val);
                (vec![], 0.0)
            }
        }
    } else {
        tracing::info!("🛒 Cart display: no user, no cookie");
        (vec![], 0.0)
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

    format::render().view(
        &v,
        "shop/cart.html",
        serde_json::json!({
            "items": items,
            "total": total,
            "current_user": user,
            "user_data": user_data,
        }),
    )
}

async fn handle_config_update(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    headers: HeaderMap,
    const_form: axum::extract::Form<ConfigUpdateForm>,
) -> Result<(CookieJar, Response)> {
    let csrf_header = headers.get("X-CSRF-Token").and_then(|v| v.to_str().ok());
    if !crate::middleware::csrf::validate_csrf(&jar, csrf_header) {
        return Err(Error::BadRequest("CSRF token inválido".to_string()));
    }

    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;
    if user.as_ref().is_none_or(|u| u.role != "admin") {
        return Err(Error::Unauthorized("Acceso denegado".to_string()));
    }

    let payload = const_form.0;
    let url_re = Regex::new(r"^https?://[a-zA-Z0-9][-a-zA-Z0-9.]*[a-zA-Z0-9](:[0-9]+)?(/.*)?$").unwrap();

    if let Some(token) = payload.token {
        if !token.is_empty() && token.len() < 8 {
            return Err(Error::BadRequest("token must be at least 8 characters".to_string()));
        }
        if !token.is_empty() {
            let config = configs::Entity::find()
                .filter(configs::Column::Key.eq("webhook_token"))
                .one(&ctx.db)
                .await?;

            if let Some(c) = config {
                let mut active_model: configs::ActiveModel = c.into();
                active_model.value = Set(Some(token));
                active_model.update(&ctx.db).await?;
            } else {
                configs::ActiveModel {
                    key: Set(Some("webhook_token".to_string())),
                    value: Set(Some(token)),
                    ..Default::default()
                }
                .insert(&ctx.db)
                .await?;
            }
            config_cache::invalidate_config_cache(&ctx, "webhook_token").await;
        }
    }

    if let Some(odoo_url) = payload.odoo_url {
        if !odoo_url.is_empty() {
            if !url_re.is_match(&odoo_url) {
                return Err(Error::BadRequest("invalid URL format (must start with http:// or https://)".to_string()));
            }
            let config = configs::Entity::find()
                .filter(configs::Column::Key.eq("odoo_base_url"))
                .one(&ctx.db)
                .await?;

            if let Some(c) = config {
                let mut active_model: configs::ActiveModel = c.into();
                active_model.value = Set(Some(odoo_url));
                active_model.update(&ctx.db).await?;
            } else {
                configs::ActiveModel {
                    key: Set(Some("odoo_base_url".to_string())),
                    value: Set(Some(odoo_url)),
                    ..Default::default()
                }
                .insert(&ctx.db)
                .await?;
            }
            config_cache::invalidate_config_cache(&ctx, "odoo_base_url").await;
        }
    }

    if let Some(ref rate) = payload.shipping_default_rate {
        if !rate.is_empty() {
            let config = configs::Entity::find()
                .filter(configs::Column::Key.eq("shipping_default_rate"))
                .one(&ctx.db)
                .await?;

            if let Some(c) = config {
                let mut active_model: configs::ActiveModel = c.into();
                active_model.value = Set(Some(rate.clone()));
                active_model.update(&ctx.db).await?;
            } else {
                configs::ActiveModel {
                    key: Set(Some("shipping_default_rate".to_string())),
                    value: Set(Some(rate.clone())),
                    ..Default::default()
                }
                .insert(&ctx.db)
                .await?;
            }
            config_cache::invalidate_config_cache(&ctx, "shipping_default_rate").await;
        }
    }

    if let Some(ref rate) = payload.shipping_local_rate {
        if !rate.is_empty() {
            let config = configs::Entity::find()
                .filter(configs::Column::Key.eq("shipping_local_rate"))
                .one(&ctx.db)
                .await?;

            if let Some(c) = config {
                let mut active_model: configs::ActiveModel = c.into();
                active_model.value = Set(Some(rate.clone()));
                active_model.update(&ctx.db).await?;
            } else {
                configs::ActiveModel {
                    key: Set(Some("shipping_local_rate".to_string())),
                    value: Set(Some(rate.clone())),
                    ..Default::default()
                }
                .insert(&ctx.db)
                .await?;
            }
            config_cache::invalidate_config_cache(&ctx, "shipping_local_rate").await;
        }
    }

    if let Some(ref key) = payload.stripe_secret_key {
        if !key.is_empty() && key.len() < 8 {
            return Err(Error::BadRequest("stripe_secret_key must be at least 8 characters".to_string()));
        }
        if !key.is_empty() {
            let config = configs::Entity::find()
                .filter(configs::Column::Key.eq("stripe_secret_key"))
                .one(&ctx.db)
                .await?;
            if let Some(c) = config {
                let mut active_model: configs::ActiveModel = c.into();
                active_model.value = Set(Some(key.clone()));
                active_model.update(&ctx.db).await?;
            } else {
                configs::ActiveModel {
                    key: Set(Some("stripe_secret_key".to_string())),
                    value: Set(Some(key.clone())),
                    ..Default::default()
                }.insert(&ctx.db).await?;
            }
            config_cache::invalidate_config_cache(&ctx, "stripe_secret_key").await;
        }
    }

    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())
        .map_err(|_| Error::string("Error al generar respuesta"))?;
    Ok((jar, response))
}

pub async fn orders_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(CookieJar, Response)> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    let Some(ref u) = user else {
        let html = std::fs::read_to_string("assets/static/403.html")
            .map_err(|_| Error::string("Error al cargar la página de acceso denegado"))?;
        let response = Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .map_err(|_| Error::string("Error al generar respuesta"))?;
        return Ok((jar, response));
    };

    let orders: Vec<orders_entity::Model> = orders_entity::Entity::find()
        .filter(orders_entity::Column::UserId.eq(u.id))
        .order_by_desc(orders_entity::Column::CreatedAt)
        .all(&ctx.db)
        .await?;

    let response = format::render().view(
        &v,
        "shop/orders.html",
        serde_json::json!({
            "current_user": user,
            "orders": orders,
        }),
    )?;

    Ok((jar, response))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/cart", get(cart_display))
        .add("/ui/auth/web-login", get(login_display))
        .add("/ui/auth/web-login", post(login_web))
        .add("/ui/auth/web-register", get(register_display))
        .add("/ui/auth/profile", get(profile_page))
        .add("/ui/auth/profile", post(update_profile))
        .add("/ui/auth/orders", get(orders_page))
        .add("/ui/auth/config", get(config_page))
        .add("/ui/auth/config", post(handle_config_update))
}
