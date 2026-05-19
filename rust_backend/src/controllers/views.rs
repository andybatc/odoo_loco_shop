#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::controllers::auth as auth_controller;
use crate::controllers::config::TokenForm;
use crate::models::_entities::{configs, users};
use crate::models::users::LoginParams;
use crate::views::auth::LoginResponse;
use axum::http::HeaderMap;
use loco_rs::auth::jwt::JWT;
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use loco_rs::prelude::Json;
use loco_rs::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct BaseContext {
    pub current_user: Option<users::Model>,
    // Aquí puedes añadir más cosas que sean globales (ej. notificaciones)
}

async fn get_current_user(ctx: &AppContext, cookie_header: Option<String>) -> Option<users::Model> {
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
        .header("HX-Redirect", "/ui/auth/token")
        .body(axum::body::Body::empty())
        .map_err(|e| Error::string(&e.to_string()))
}

async fn register_display() -> Result<Response> {
    let html = std::fs::read_to_string("assets/views/auth/register.html")
        .map_err(|_| Error::string("No se encuentra la plantilla de registro"))?;
    format::html(&html)
}

pub async fn render_ui(
    // 1. EXTRAEMOS EL MOTOR DE VISTAS AQUÍ (Axum te lo da mágicamente)
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    // Obtener el token de la DB
    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq("webhook_token"))
        .one(&ctx.db)
        .await
        .map_err(|e| {
            tracing::error!("Error consultando la DB: {:?}", e);
            Error::string("Error al conectar con la base de datos")
        })?;

    let token_value = config
        .and_then(|c| c.value)
        .unwrap_or_else(|| "No configurado".to_string());

    // Obtener usuario para el header
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    // 2. USAMOS .view() EN LUGAR DE .template()
    // Le pasamos la referencia al motor de vistas (&v)
    format::render().view(
        &v,
        "config/ui.html",
        serde_json::json!({
            "current_user": user,
            "current_token": token_value,
            "mensaje": "Vue está funcionando"
        }),
    )
}

async fn handle_ui_update(
    State(ctx): State<AppContext>,
    // Usamos Form en lugar de Json para capturar el envío del navegador
    const_form: axum::extract::Form<TokenForm>,
) -> Result<Response> {
    let payload = const_form.0;

    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq("webhook_token"))
        .one(&ctx.db)
        .await?;

    if let Some(c) = config {
        let mut active_model: configs::ActiveModel = c.into();
        active_model.value = Set(Some(payload.token));
        active_model.update(&ctx.db).await?;
    } else {
        configs::ActiveModel {
            key: Set(Some("webhook_token".to_string())),
            value: Set(Some(payload.token)),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await?;
    }

    Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())
        .map_err(|e| Error::string(&e.to_string()))
    // Después de guardar, refrescamos la página
    //format::render().redirect("ui/auth/token")
}

pub fn routes() -> Routes {
    Routes::new()
        // Grupo para Autenticación Web
        // URL Resultante: /api/ui/auth/web-login
        .prefix("ui/auth")
        .add("/web-login", get(login_display))
        .add("/web-login", post(login_web))
        .add("/web-register", get(register_display))
        // Grupo para Configuración
        // URL Resultante: /api/views/config/token
        .add("/token", get(render_ui))
        .add("/token", post(handle_ui_update))
}
