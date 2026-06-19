#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::controllers::auth as auth_controller;
use crate::models::_entities::{users, cart_items, carts, products, configs};
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
use axum_extra::extract::cookie::CookieJar;

#[derive(Serialize)]
pub struct BaseContext {
    pub current_user: Option<users::Model>,
    // Aquí puedes añadir más cosas que sean globales (ej. notificaciones)
}

#[derive(serde::Serialize)]
pub struct CartItemRender {
    pub id: i32,
    pub name: String,
    pub price: f64,
    pub quantity: i32,
    pub subtotal: f64,
    pub image_filename: Option<String>,
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
    // Esto te dirá en la terminal desde dónde se está ejecutando el programa
    if let Ok(current_dir) = std::env::current_dir() {
        println!("Directorio actual de ejecución: {:?}", current_dir);
    }
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));

    // 2. Obtener el usuario (si existe)
    let user = get_current_user(&ctx, cookie_header).await;

    let html_path = "assets/views/auth/login.html";
    let html = std::fs::read_to_string(html_path).map_err(|e| {
        // Imprime el error real de sistema (ej. Permission Denied o No such file)
        println!("Error leyendo el HTML ({}) : {:?}", html_path, e);
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
        .map_err(|e| Error::string(&e.to_string()))?;

    let login_res: LoginResponse = serde_json::from_slice(&body_bytes)
        .map_err(|_| Error::string("Error al procesar respuesta de autenticación"))?;

    // --- MANEJO DE COOKIES ---
    let jwt_config = ctx.config.get_jwt_config()?;
    let cookie = format!(
        "token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        login_res.token, jwt_config.expiration
    );

    // Respondemos al navegador
    Response::builder()
        .header("Set-Cookie", cookie)
        .header("HX-Redirect", "/ui/auth/config")
        .body(axum::body::Body::empty())
        .map_err(|e| Error::string(&e.to_string()))
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
) -> Result<Response> {
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

    format::render().view(
        &v,
        "config/ui.html",
        serde_json::json!({
            "current_user": user,
            "current_token": token_value,
            "odoo_base_url": odoo_url_value,
        }),
    )
}

pub async fn cart_display(
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

    // 1. Intentar recuperar el carrito mediante la cookie de sesión
    if let Some(cookie) = jar.get(cookie_name) {
        if let Ok(cart_uuid) = Uuid::parse_str(cookie.value()) {

            // 2. Obtener todos los ítems vinculados a este carrito
            let items = cart_items::Entity::find()
                .filter(cart_items::Column::CartId.eq(cart_uuid))
                .all(&ctx.db)
                .await?;

            if !items.is_empty() {
                let mut product_ids = Vec::new();
                let mut item_quantities = std::collections::HashMap::new();

                // 3. Revertir el padding del UUID para obtener los IDs numéricos (i32) de Odoo
                for item in items {
                    let hex_str = item.product_id.simple().to_string();
                    if let Ok(prod_id) = i32::from_str_radix(&hex_str, 16) {
                        product_ids.push(prod_id);
                        item_quantities.insert(prod_id, item.quantity);
                    }
                }

                // 4. Buscar los productos en la base de datos de una sola consulta
                let db_products = products::Entity::find()
                    .filter(products::Column::Id.is_in(product_ids))
                    .all(&ctx.db)
                    .await?;

                // 5. Armar el contexto listo para la vista
                for prod in db_products {
                    let qty = *item_quantities.get(&prod.id).unwrap_or(&1);
                    let price_f64 = prod.price.map(|p| p.to_string().parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);
                    let subtotal = price_f64 * (qty as f64);
                    grand_total += subtotal;

                    render_items.push(CartItemRender {
                        id: prod.id,
                        name: prod.name.unwrap_or_else(|| "Producto sin nombre".to_string()),
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
        "shop/cart.html",
        &serde_json::json!({
            "items": render_items,
            "total": grand_total,
            "current_user": user,
        }),
    )
}

async fn handle_config_update(
    State(ctx): State<AppContext>,
    const_form: axum::extract::Form<ConfigUpdateForm>,
) -> Result<Response> {
    let payload = const_form.0;

    if let Some(token) = payload.token {
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

    Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())
        .map_err(|e| Error::string(&e.to_string()))
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
