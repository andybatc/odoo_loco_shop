#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use axum::extract::Query;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::_entities::products;

#[derive(Debug, Deserialize)]
pub struct ShippingEstimateParams {
    pub product_ids: String,
    pub country: String,
    pub state: String,
}

#[debug_handler]
pub async fn estimate(
    State(ctx): State<AppContext>,
    Query(params): Query<ShippingEstimateParams>,
) -> Result<Response> {
    let ids: Vec<i32> = params
        .product_ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let items = products::Entity::find()
        .filter(products::Column::Id.is_in(ids))
        .all(&ctx.db)
        .await
        .unwrap_or_default();

    let (shipping_cost, origin_summary) = crate::controllers::checkout::calc_shipping(
        &ctx.db,
        &items,
        &params.country,
        &params.state,
    )
    .await
    .unwrap_or((sea_orm::prelude::Decimal::ZERO, String::new()));

    format::json(serde_json::json!({
        "shipping_cost": shipping_cost,
        "origin_summary": origin_summary,
    }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/shipping")
        .add("/estimate", get(estimate))
}
