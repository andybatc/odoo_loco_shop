#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::middleware::auth_extractor::AdminUser;
use crate::models::_entities::orders;
use loco_rs::controller::views::engines::TeraView;
use loco_rs::controller::views::ViewEngine;
use loco_rs::prelude::*;
use sea_orm::{query::*, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OrderListParams {
    pub page: Option<u32>,
    pub status: Option<String>,
}

pub async fn admin_dashboard(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    admin: AdminUser,
) -> Result<Response> {
    let total_orders = orders::Entity::find()
        .count(&ctx.db)
        .await
        .unwrap_or(0);

    let pending_orders = orders::Entity::find()
        .filter(orders::Column::Status.eq("pending"))
        .count(&ctx.db)
        .await
        .unwrap_or(0);

    let confirmed_orders = orders::Entity::find()
        .filter(orders::Column::Status.eq("confirmed"))
        .count(&ctx.db)
        .await
        .unwrap_or(0);

    let failed_orders = orders::Entity::find()
        .filter(orders::Column::Status.eq("failed"))
        .count(&ctx.db)
        .await
        .unwrap_or(0);

    let recent_orders = orders::Entity::find()
        .order_by_desc(orders::Column::CreatedAt)
        .limit(10)
        .all(&ctx.db)
        .await
        .unwrap_or_default();

    format::render().view(
        &v,
        "admin/dashboard.html",
        serde_json::json!({
            "current_user": admin.user,
            "total_orders": total_orders,
            "pending_orders": pending_orders,
            "confirmed_orders": confirmed_orders,
            "failed_orders": failed_orders,
            "recent_orders": recent_orders,
        }),
    )
}

pub async fn order_list(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    admin: AdminUser,
    Query(params): Query<OrderListParams>,
) -> Result<Response> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page: u64 = 20;
    let offset = ((page - 1) as u64) * per_page;

    let mut base_query = orders::Entity::find()
        .order_by_desc(orders::Column::CreatedAt);

    if let Some(ref status) = params.status {
        if !status.is_empty() {
            base_query = base_query.filter(orders::Column::Status.eq(status));
        }
    }

    let total = base_query.clone().count(&ctx.db).await.unwrap_or(0);
    let total_pages = (total as f64 / per_page as f64).ceil() as u32;

    let all_orders = base_query
        .clone()
        .offset(offset)
        .limit(per_page)
        .all(&ctx.db)
        .await?;

    format::render().view(
        &v,
        "admin/orders.html",
        serde_json::json!({
            "current_user": admin.user,
            "orders": all_orders,
            "page": page,
            "total_pages": total_pages,
            "current_status": params.status,
        }),
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("admin")
        .add("/dashboard", get(admin_dashboard))
        .add("/orders", get(order_list))
}
