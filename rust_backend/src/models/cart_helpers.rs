use crate::models::_entities::{cart_items, products};
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct CartItemRender {
    pub id: i32,
    pub item_id: Uuid,
    pub name: String,
    pub price: f64,
    pub quantity: i32,
    pub subtotal: f64,
    pub image_filename: Option<String>,
    pub tax_percent: Option<String>,
}

#[derive(Serialize)]
pub struct CartWithTotal {
    pub items: Vec<CartItemRender>,
    pub total: f64,
}

pub async fn load_cart(ctx: &AppContext, cart_uuid: Uuid) -> Result<CartWithTotal> {
    let items = cart_items::Entity::find()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .all(&ctx.db)
        .await?;

    if items.is_empty() {
        return Ok(CartWithTotal { items: vec![], total: 0.0 });
    }

    let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
    let mut item_quantities = std::collections::HashMap::new();
    let mut item_ids = std::collections::HashMap::new();
    for item in &items {
        item_quantities.insert(item.product_id, item.quantity);
        item_ids.insert(item.product_id, item.id);
    }

    let db_products = products::Entity::find()
        .filter(products::Column::Id.is_in(product_ids))
        .all(&ctx.db)
        .await?;

    let mut grand_total = 0.0;
    let mut render_items = Vec::new();

    for prod in db_products {
        let qty = *item_quantities.get(&prod.id).unwrap_or(&1);
        let price_f64 = prod.price.map(|p| p.to_string().parse::<f64>().unwrap_or(0.0)).unwrap_or(0.0);
        let subtotal = price_f64 * (qty as f64);
        grand_total += subtotal;

        render_items.push(CartItemRender {
            id: prod.id,
            item_id: *item_ids.get(&prod.id).unwrap_or(&Uuid::default()),
            name: prod.name.unwrap_or_else(|| "Producto sin nombre".to_string()),
            price: price_f64,
            quantity: qty,
            subtotal,
            image_filename: prod.image_filename,
            tax_percent: prod.tax_percent.map(|t| t.to_string()),
        });
    }

    Ok(CartWithTotal { items: render_items, total: grand_total })
}
