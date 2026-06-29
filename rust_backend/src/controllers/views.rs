#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::controllers::auth as auth_controller;
use crate::models::_entities::{users, configs};
use crate::models::cart_helpers;
use crate::models::config_cache;
use crate::models::users::LoginParams;
use crate::views::auth::LoginResponse;
use axum::http::HeaderMap;
use loco_rs::auth::jwt::JWT;
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use loco_rs::prelude::Json;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use regex::Regex;

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
    Form(params): Form<LoginParams>, // Recibimos el Formulario del HTML
) -> Result<Response> {
    // --- EL PUENTE ---
    // Convertimos el Form<LoginParams> en Json<LoginParams> para dárselo a Loco
    let login_json = Json(params);

    // Llamamos directamente a la función 'login' del controlador de Loco
    let api_response = auth_controller::login(State(ctx.clone()), login_json).await?;

    // --- PROCESAR LA RESPUESTA DE LOCO ---
    // Si llegamos aquí, el login fue exitoso (Loco devolvió un Ok)
    // Extraemos el cuerpo de la respuesta para obtener el Token
    // Nota: Loco devuelve LoginResponse en formato JSON
    let body_bytes = axum::body::to_bytes(api_response.into_body(), 1024 * 10)
        .await
        .map_err(|_| Error::string("Error al leer respuesta de autenticación"))?;

    let login_res: LoginResponse = serde_json::from_slice(&body_bytes)
        .map_err(|_| Error::string("Error al procesar respuesta de autenticación"))?;

    // --- MANEJO DE COOKIES ---
    let jwt_config = ctx.config.get_jwt_config()?;
    let cookie = format!(
        "token={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        login_res.token, jwt_config.expiration
    );

    // Respondemos al navegador
    Response::builder()
        .header("Set-Cookie", cookie)
        .header("HX-Redirect", "/")
        .body(axum::body::Body::empty())
        .map_err(|_| Error::string("Error al generar respuesta de autenticación"))
}

async fn register_display() -> Result<Response> {
    let html = std::fs::read_to_string("assets/views/auth/register.html")
        .map_err(|_| Error::string("No se encuentra la plantilla de registro"))?;
    format::html(&html)
}

#[derive(Deserialize)]
pub struct ConfigUpdateForm {
    pub token: Option<String>,
    pub odoo_url: Option<String>,
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

    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    if user.as_ref().map_or(true, |u| u.role != "admin") {
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
            "csrf_token": csrf_token,
        }),
    )?;

    Ok((jar, response))
}

pub async fn cart_display(
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

    format::render().view(
        &v,
        "shop/cart.html",
        &serde_json::json!({
            "items": items,
            "total": total,
            "current_user": user,
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

    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())
        .map_err(|_| Error::string("Error al generar respuesta"))?;
    Ok((jar, response))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/cart", get(cart_display))
        // Grupo para Autenticación Web
        // URL Resultante: /api/ui/auth/web-login
        .prefix("ui/auth")
        .add("/web-login", get(login_display))
        .add("/web-login", post(login_web))
        .add("/web-register", get(register_display))
        // Grupo para Configuración
        // URL Resultante: /api/ui/auth/config
        .add("/config", get(config_page))
        .add("/config", post(handle_config_update))
}
