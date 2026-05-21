#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::models::product_template_odoo::{self};
use loco_rs::prelude::*;
use sea_orm::{query::*, Database,ColumnTrait, QueryFilter};
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use axum::http::HeaderMap;
use crate::models::_entities::products;
use crate::controllers::views::get_current_user;

#[debug_handler]
pub async fn list(State(_ctx): State<AppContext>) -> Result<Response> {
    // 1. Conexión a Odoo
    // Nota: Más adelante moveremos esto a un recurso global para no conectar en cada request
    let odoo_db = Database::connect("postgres://odoo:postgres@localhost:5432/odoo_prod")
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?;

    // 2. Consulta
    let products = product_template_odoo::Entity::find()
        .filter(product_template_odoo::Column::IsPublished.eq(true))
        .limit(10)
        .all(&odoo_db)
        .await?;

    format::json(products)
}
pub async fn index(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap
) -> Result<Response> {

    // 1. Buscamos los productos
    let products = products::Entity::find()
        .filter(products::Column::IsPublished.eq(true))
        .all(&ctx.db)
        .await?;

    let cookie_header = headers.get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;
    // 3. Pasamos &v como el primer argumento
    format::render().view(
        &v,
        "shop/home.html",
        serde_json::json!({
            "products": products,
            "current_user": user,
        }),
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/shop")
        // Esta es la ruta para ver tu vista con Vue (renderiza el HTML)
        .add("/home", get(index))
        // Esta es tu API, puedes dejarla así o cambiar el prefijo para que no choquen
        .add("/api/products", get(list))
}
