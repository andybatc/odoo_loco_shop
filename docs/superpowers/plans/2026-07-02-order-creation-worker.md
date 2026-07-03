# Order Creation Worker — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Odoo order creation to a Redis-backed background worker so checkout responds instantly, with 3 retry attempts.

**Architecture:** New `OrderCreationWorker` (BackgroundWorker) recibe `order_id`, carga orden+items de DB, hace HTTP a Odoo con retry. Checkout handler se simplifica: crea orden, encola worker, responde inmediato.

**Tech Stack:** Loco.rs BackgroundWorker (Redis queue), reqwest, SeaORM

## Global Constraints

- Usar `loco_rs::prelude::*`, `sea_orm::ActiveValue::Set`
- `reqwest` ya está en Cargo.toml — no agregar dependencias
- Seguir patrón de `workers/product_sync.rs` y `workers/webhook.rs`
- `order.status` = `"pending"` → `"confirmed"` | `"failed"`
- No cambiar modelos DB ni endpoint Odoo Python

---

### Task 1: Crear `src/workers/order_creation.rs`

**Files:**
- Create: `src/workers/order_creation.rs`

**Interfaces:**
- Consumes: `AppContext`, `OrderWorkerArgs { order_id: Uuid }`
- Produces: struct `OrderCreationWorker` implementando `BackgroundWorker<OrderWorkerArgs>`

- [ ] **Step 1: Write scaffold con import y structs**

```rust
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
```

- [ ] **Step 2: Implementar `BackgroundWorker`**

```rust
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
            .ok_or_else(|| Error::msg(format!("Orden {} no encontrada", args.order_id)))?;

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

        // Construir payload para Odoo (misma estructura que checkout.rs)
        let odoo_items: Vec<serde_json::Value> = items.iter().map(|item| {
            let price_f64 = item.price.to_string().parse::<f64>().unwrap_or(0.0);
            serde_json::json!({
                "product_id": item.product_id,
                "name": item.product_name,
                "price": price_f64,
                "quantity": item.quantity,
            })
        }).collect();

        let mut payload = serde_json::json!({
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

        // Leer webhook_token + odoo_base_url de configs
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

        // 3 intentos con delays
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
                .map_err(|e| Error::msg(e.to_string()))?;

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

                    let confirmed = orders_entity::ActiveModel {
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

        // 3 fallos
        tracing::error!("❌ Orden {} falló tras {} intentos", args.order_id, max_retries);
        orders_entity::ActiveModel {
            id: Set(args.order_id),
            status: Set("failed".to_string()),
            ..Default::default()
        }.update(&self.ctx.db).await?;

        Ok(())
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/workers/order_creation.rs
git commit -m "feat: add OrderCreationWorker with retry logic"
```

---

### Task 2: Simplificar `submit_checkout` en checkout.rs

**Files:**
- Modify: `src/controllers/checkout.rs`

**Interfaces:**
- Consumes: `OrderWorkerArgs { order_id }` from Task 1, `OrderCreationWorker::perform_later`
- Produces: Simplified `submit_checkout` that enqueues worker instead of calling Odoo

- [ ] **Step 1: Agregar import del worker**

```rust
use crate::workers::order_creation::{OrderCreationWorker, OrderWorkerArgs};
```

Agregar al bloque de imports existente.

- [ ] **Step 2: Reemplazar bloque HTTP a Odoo por enqueue del worker**

En `submit_checkout`, después de insertar order_items (L251), **eliminar** todo desde la lectura de configs (L253) hasta el cierre del match (L400).

**Reemplazar** con:

```rust
    // Encolar worker de creación en Odoo
    let worker_args = OrderWorkerArgs { order_id };
    OrderCreationWorker::perform_later(&ctx, worker_args).await?;

    // Limpiar carrito
    cart_items::Entity::delete_many()
        .filter(cart_items::Column::CartId.eq(cart_uuid))
        .exec(&ctx.db)
        .await?;
    carts::Entity::delete_by_id(cart_uuid)
        .exec(&ctx.db)
        .await?;

    let jar = jar.remove(Cookie::new(cookie_name, ""));

    Ok((
        jar,
        Json(CheckoutResponse {
            success: true,
            order_name: None,
            invoice_name: None,
            total: Some(total_f64),
            error: None,
        }),
    ))
```

- [ ] **Step 3: Limpiar imports no usados**

Remover estos imports si quedan sin uso tras el cambio:
- `configs` (puede seguir usado en checkout_page? No, checkout_page no lo usa. Pero `std::time::Duration` sí se usa en cache expiry L82)

Revisar warnings de `cargo check`.

- [ ] **Step 4: Commit**

```bash
git add src/controllers/checkout.rs
git commit -m "refactor: submit_checkout ahora encola OrderCreationWorker"
```

---

### Task 3: Registrar worker en app.rs

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Agregar registro del worker**

En `connect_workers` (L154-158), agregar:

```rust
queue.register(crate::workers::order_creation::OrderCreationWorker::build(ctx)).await?;
```

entre los registros existentes.

- [ ] **Step 2: Commit**

```bash
git add src/app.rs
git commit -m "feat: register OrderCreationWorker in app.rs"
```

---

### Task 4: Build y verificar

- [ ] **Step 1: Verificar que compila**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Build completo**

```bash
cargo build 2>&1 | tail -3
```

Expected: `Finished` sin errores.

- [ ] **Step 3: Verificar warnings sin importancia**

```bash
cargo check 2>&1 | grep "^warning" | grep -v "migration" | grep -v "unused import"
```

Solo los warnings pre-existentes. Si hay nuevos warnings (e.g. imports no usados), limpiarlos.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: build verification and import cleanup"
```
