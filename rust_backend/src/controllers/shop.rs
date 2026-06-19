#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::models::product_template_odoo::{self};
use loco_rs::prelude::*;
use sea_orm::{query::*, Database,ColumnTrait, QueryFilter};
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use axum::http::HeaderMap;
use axum::extract::Query;
use crate::models::_entities::products;
use crate::models::products as product_model;
use crate::controllers::views::get_current_user;
use serde::{Deserialize, Serialize};

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
    let cache_key = "products:all";

    let products = match ctx.cache.get::<Vec<products::Model>>(cache_key).await {
        Ok(Some(cached_products)) => {
            tracing::info!("⚡ Hit de Caché: Catálogo cargado desde Redis");
            cached_products
        }
        _ => {
            tracing::info!("🐢 Cache Miss: Cargando catálogo desde Postgres...");
            let db_products = products::Entity::find()
                .filter(products::Column::IsPublished.eq(true))
                .order_by_asc(products::Column::Name)
                .all(&ctx.db)
                .await?;

            let _ = ctx.cache.insert(cache_key, &db_products).await;

            db_products
        }
    };

    let cookie_header = headers.get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    format::render().view(
        &v,
        "shop/home.html",
        serde_json::json!({
            "products": products,
            "current_user": user,
        }),
    )
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub page: Option<u32>,
}

#[debug_handler]
pub async fn search_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(params): Query<SearchParams>,
    headers: HeaderMap,
) -> Result<Response> {
    let query = params.q.unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let page_size = 12;

    let (products, total) = if query.trim().is_empty() {
        (Vec::new(), 0)
    } else {
        product_model::Entity::search_products(&ctx.db, &query, page, page_size).await?
    };

    let total_pages = if page_size > 0 { (total as f64 / page_size as f64).ceil() as u64 } else { 1 };

    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    format::render().view(
        &v,
        "shop/search.html",
        serde_json::json!({
            "products": products,
            "query": query,
            "page": page,
            "total_pages": total_pages,
            "total": total,
            "current_user": user,
        }),
    )
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub id: i32,
    pub name: String,
    pub price: Option<String>,
    pub image_filename: Option<String>,
}

#[debug_handler]
pub async fn search_api(
    State(ctx): State<AppContext>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResultItem>>> {
    let query = params.q.unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let page_size = 12;

    if query.trim().is_empty() {
        return Ok(Json(Vec::new()));
    }

    let (products, _total) =
        product_model::Entity::search_products(&ctx.db, &query, page, page_size).await?;

    let items: Vec<SearchResultItem> = products
        .into_iter()
        .map(|p| SearchResultItem {
            id: p.id,
            name: p.name.unwrap_or_default(),
            price: p.price.map(|d| d.to_string()),
            image_filename: p.image_filename,
        })
        .collect();

    Ok(Json(items))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/shop")
        .add("/home", get(index))
        .add("/search", get(search_page))
        .add("/api/search", get(search_api))
        .add("/api/products", get(list))
}
