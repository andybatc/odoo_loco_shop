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

    let cached = get_or_fetch_search(&ctx, &query, page, page_size).await?;

    let total_pages = if page_size > 0 {
        (cached.total as f64 / page_size as f64).ceil() as u64
    } else {
        1
    };

    let cookie_header = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok().map(|s| s.to_string()));
    let user = get_current_user(&ctx, cookie_header).await;

    format::render().view(
        &v,
        "shop/search.html",
        serde_json::json!({
            "products": cached.items,
            "query": query,
            "page": page,
            "total_pages": total_pages,
            "total": cached.total,
            "current_user": user,
        }),
    )
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchResultItem {
    pub id: i32,
    pub name: String,
    pub sku: Option<String>,
    pub price: Option<String>,
    pub image_filename: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedSearchResult {
    pub items: Vec<SearchResultItem>,
    pub total: u64,
}

async fn get_search_version(ctx: &AppContext) -> i64 {
    ctx.cache
        .get::<i64>("products:search_version")
        .await
        .ok()
        .flatten()
        .unwrap_or(1)
}

fn normalize_query(q: &str) -> String {
    q.trim().to_lowercase()
}

fn search_cache_key(ver: i64, query: &str, page: u32) -> String {
    format!("search:v{}:{}:{}", ver, query, page)
}

async fn fetch_and_cache_search(
    ctx: &AppContext,
    query: &str,
    page: u32,
    page_size: u32,
    ver: i64,
) -> Result<CachedSearchResult> {
    let (products, total) =
        product_model::Entity::search_products(&ctx.db, query, page, page_size).await?;

    let items: Vec<SearchResultItem> = products
        .into_iter()
        .map(|p| SearchResultItem {
            id: p.id,
            name: p.name.clone().unwrap_or_default(),
            sku: p.sku.clone(),
            price: p.price.map(|d| d.to_string()),
            image_filename: p.image_filename.clone(),
        })
        .collect();

    let cached = CachedSearchResult { items, total };
    let cache_key = search_cache_key(ver, &normalize_query(query), page);
    let _ = ctx.cache.insert_with_expiry(&cache_key, &cached, std::time::Duration::from_secs(300)).await;

    Ok(cached)
}

async fn get_or_fetch_search(
    ctx: &AppContext,
    query: &str,
    page: u32,
    page_size: u32,
) -> Result<CachedSearchResult> {
    if normalize_query(query).is_empty() {
        return Ok(CachedSearchResult {
            items: Vec::new(),
            total: 0,
        });
    }

    let ver = get_search_version(ctx).await;
    let cache_key = search_cache_key(ver, &normalize_query(query), page);

    if let Ok(Some(cached)) = ctx.cache.get::<CachedSearchResult>(&cache_key).await {
        tracing::info!("⚡ Cache hit: búsqueda '{}' página {}", query, page);
        return Ok(cached);
    }

    tracing::info!("🐢 Cache miss: búsqueda '{}' página {}", query, page);
    fetch_and_cache_search(ctx, query, page, page_size, ver).await
}

#[debug_handler]
pub async fn search_api(
    State(ctx): State<AppContext>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResultItem>>> {
    let query = params.q.unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let page_size = 12;

    let cached = get_or_fetch_search(&ctx, &query, page, page_size).await?;
    Ok(Json(cached.items))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/shop")
        .add("/home", get(index))
        .add("/search", get(search_page))
        .add("/api/search", get(search_api))
        .add("/api/products", get(list))
}
