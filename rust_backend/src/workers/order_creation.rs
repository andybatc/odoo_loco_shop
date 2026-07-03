use crate::models::_entities::{configs, order_items, orders as orders_entity};
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::Duration;

pub struct OrderCreationWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct OrderWorkerArgs {
    pub order_id: Uuid,
}

#[async_trait]
impl BackgroundWorker<OrderWorkerArgs> for OrderCreationWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "OrderCreation".to_string()
    }

    async fn perform(&self, args: OrderWorkerArgs) -> Result<()> {
        tracing::info!("🧾 Procesando orden: {}", args.order_id);

        let order = orders_entity::Entity::find_by_id(args.order_id)
            .one(&self.ctx.db)
            .await?
            .ok_or_else(|| {
                let msg = format!("Orden {} no encontrada", args.order_id);
                Error::string(&msg)
            })?;

        if order.status != "pending" {
            tracing::warn!("Orden {} ya no está pendiente (status={}), saltando", args.order_id, order.status);
            return Ok(());
        }

        let items = order_items::Entity::find()
            .filter(order_items::Column::OrderId.eq(args.order_id))
            .all(&self.ctx.db)
            .await?;

        if items.is_empty() {
            tracing::warn!("Orden {} sin items, marcando como failed", args.order_id);
            orders_entity::ActiveModel {
                id: Set(args.order_id),
                status: Set("failed".to_string()),
                ..Default::default()
            }.update(&self.ctx.db).await?;
            return Ok(());
        }

        let odoo_items: Vec<serde_json::Value> = items.iter().map(|item| {
            let price_f64 = item.price.to_string().parse::<f64>().unwrap_or(0.0);
            serde_json::json!({
                "product_id": item.product_id,
                "name": item.product_name,
                "price": price_f64,
                "quantity": item.quantity,
            })
        }).collect();

        let payload = serde_json::json!({
            "customer": {
                "name": order.customer_name,
                "email": order.customer_email,
                "phone": order.customer_phone,
                "street": order.customer_street,
                "city": order.customer_city,
                "zip": order.customer_zip,
            },
            "items": odoo_items,
        });

        let config = configs::Entity::find()
            .filter(configs::Column::Key.eq("webhook_token"))
            .one(&self.ctx.db)
            .await?;
        let token = config.and_then(|c| c.value).unwrap_or_default();

        let odoo_domain = configs::Entity::find()
            .filter(configs::Column::Key.eq("odoo_base_url"))
            .one(&self.ctx.db)
            .await?
            .and_then(|c| c.value)
            .unwrap_or_else(|| "http://localhost:8072".to_string());

        let odoo_url = format!("{}/api/orders/create", odoo_domain);

        let delays = [Duration::from_secs(0), Duration::from_secs(30), Duration::from_secs(300)];
        let max_retries = delays.len();

        for attempt in 0..max_retries {
            if attempt > 0 {
                tracing::info!("Reintento {} para orden {}", attempt + 1, args.order_id);
                tokio::time::sleep(delays[attempt]).await;
            }

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| Error::msg(e))?;

            match client
                .post(&odoo_url)
                .header("Authorization", format!("Bearer {}", token))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        tracing::warn!("Intento {}: Odoo respondió {}: {}", attempt + 1, status, body);
                        continue;
                    }

                    let odoo_resp: serde_json::Value = resp.json().await.unwrap_or_default();

                    if odoo_resp.get("error").is_some() {
                        tracing::warn!("Intento {}: Odoo devolvió error: {:?}", attempt + 1, odoo_resp["error"]);
                        continue;
                    }

                    let order_name = odoo_resp["order_name"].as_str().map(|s| s.to_string());
                    let invoice_name = odoo_resp["invoice_name"].as_str().map(|s| s.to_string());

                    orders_entity::ActiveModel {
                        id: Set(args.order_id),
                        status: Set("confirmed".to_string()),
                        odoo_order_name: Set(order_name),
                        odoo_invoice_name: Set(invoice_name),
                        ..Default::default()
                    }.update(&self.ctx.db).await?;

                    tracing::info!("✅ Orden {} confirmada en Odoo", args.order_id);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Intento {}: error de conexión: {}", attempt + 1, e);
                }
            }
        }

        tracing::error!("❌ Orden {} falló tras {} intentos", args.order_id, max_retries);
        orders_entity::ActiveModel {
            id: Set(args.order_id),
            status: Set("failed".to_string()),
            ..Default::default()
        }.update(&self.ctx.db).await?;

        Ok(())
    }
}
