#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use crate::controllers::views::get_current_user;

pub async fn home(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;
    format::render().view(
        &v,
        "homepages/home.html",
        serde_json::json!({
            "current_user": user,
            "titulo": "¡Bienvenido a nuestra Tienda!",
            "descripcion": "Sincronizada en tiempo real con Odoo"
        })
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("")
        .add("/", get(home))
}