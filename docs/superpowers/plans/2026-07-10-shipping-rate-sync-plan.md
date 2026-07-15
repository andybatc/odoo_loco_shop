# Shipping Rate Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Sincronizar tarifas de envío desde Odoo al backend Rust en tiempo real (create/write) + manual (server action).

**Architecture:** Endpoint HTTP + BackgroundWorker en Rust/Loco. Odoo `shipping.rate` envía tarifas al endpoint via Bearer token. Rust upserta por unique key en `shipping_rates`.

**Tech Stack:** Rust/Loco (Redis queue), SeaORM, Odoo 18, Python requests

**Global Constraints:**

- El endpoint reusa `AuthToken` extractor de `middleware/auth_extractor.rs` (Bearer token de `webhook_token`)
- Worker usa `BackgroundWorker` trait de Loco (Redis queue)
- Odoo envía nombres de país en inglés (`with_context(lang='en_US')`)
- no bloquear create/write si el HTTP call falla (try/except)
- Seguir patrones existentes en `workers/webhook.rs` y `controllers/shipping.rs`

---

### Task 1: Rust worker — `ShippingRateSyncWorker`

**Files:**
- Create: `src/workers/shipping_rate_sync.rs`
- Modify: `src/workers/mod.rs`

**Interfaces:**
- Produces: `ShippingRateSyncWorker` (impl `BackgroundWorker<ShippingRateSyncArgs>`), `ShippingRateSyncArgs`, `RatePayload`

**Steps:**

- [x] **Step 1: Crear archivo worker**

```rust
// src/workers/shipping_rate_sync.rs
use crate::models::_entities::shipping_rates;
use loco_rs::bgworker::BackgroundWorker;
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct RatePayload {
    pub origin_country: String,
    pub origin_state: String,
    pub dest_country: String,
    pub dest_state: String,
    pub amount: f64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ShippingRateSyncArgs {
    pub rates: Vec<RatePayload>,
}

pub struct ShippingRateSyncWorker {
    pub ctx: AppContext,
}

#[async_trait]
impl BackgroundWorker<ShippingRateSyncArgs> for ShippingRateSyncWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "ShippingRateSync".to_string()
    }

    async fn perform(&self, args: ShippingRateSyncArgs) -> Result<()> {
        let now = chrono::Utc::now().into();
        for rate in &args.rates {
            let existing = shipping_rates::Entity::find()
                .filter(shipping_rates::Column::OriginCountry.eq(&rate.origin_country))
                .filter(shipping_rates::Column::OriginState.eq(&rate.origin_state))
                .filter(shipping_rates::Column::DestCountry.eq(&rate.dest_country))
                .filter(shipping_rates::Column::DestState.eq(&rate.dest_state))
                .one(&self.ctx.db)
                .await?;

            let amount =
                sea_orm::prelude::Decimal::try_from(rate.amount).unwrap_or(sea_orm::prelude::Decimal::ZERO);

            if let Some(existing) = existing {
                let mut am: shipping_rates::ActiveModel = existing.into();
                am.amount = Set(amount);
                am.updated_at = Set(now);
                am.update(&self.ctx.db).await?;
            } else {
                shipping_rates::ActiveModel {
                    origin_country: Set(rate.origin_country.clone()),
                    origin_state: Set(rate.origin_state.clone()),
                    dest_country: Set(rate.dest_country.clone()),
                    dest_state: Set(rate.dest_state.clone()),
                    amount: Set(amount),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&self.ctx.db)
                .await?;
            }
        }
        tracing::info!("Synced {} shipping rates via worker", args.rates.len());
        Ok(())
    }
}
```

- [x] **Step 2: Registrar módulo en `src/workers/mod.rs`**

```rust
pub mod shipping_rate_sync;
```

Añadir entre `pub mod shipping_sync;` y la línea siguiente.

- [x] **Step 3: Compilar para verificar**

```bash
cd /home/andy/Proyectos/odoo_loco_shop/rust_backend && cargo check 2>&1 | tail -5
```



---

### Task 2: Rust endpoint `POST /api/shipping/rates/sync`

**Files:**
- Modify: `src/controllers/shipping.rs`
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `RatePayload` de Task 1
- Produces: `POST /api/shipping/rates/sync` route

**Steps:**

- [x] **Step 1: Agregar import y handler a `src/controllers/shipping.rs`**

Agregar al inicio (después de `use serde::Deserialize;`):
```rust
use crate::workers::shipping_rate_sync::ShippingRateSyncWorker;
use loco_rs::bgworker::BackgroundWorker;
```

Agregar antes de `pub fn routes()`:
```rust
#[derive(Debug, Deserialize)]
pub struct SyncRatesRequest {
    pub rates: Vec<crate::workers::shipping_rate_sync::RatePayload>,
}

#[debug_handler]
pub async fn sync_rates(
    State(ctx): State<AppContext>,
    _auth: crate::middleware::auth_extractor::AuthToken,
    Json(payload): Json<SyncRatesRequest>,
) -> Result<Response> {
    if payload.rates.is_empty() {
        return Ok(Json(serde_json::json!({
            "status": "error",
            "message": "No rates provided"
        })));
    }

    ShippingRateSyncWorker::perform_later(
        &ctx,
        crate::workers::shipping_rate_sync::ShippingRateSyncArgs { rates: payload.rates },
    )
    .await?;

    Ok(Json(serde_json::json!({"status": "accepted"})))
}
```

- [x] **Step 2: Agregar ruta en `pub fn routes()`**

```rust
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/shipping")
        .add("/estimate", get(estimate))
        .add("/rates/sync", post(sync_rates))
}
```

- [x] **Step 3: Registrar worker en `src/app.rs` `connect_workers`**

```rust
async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
    queue.register(crate::workers::webhook::WebhookWorker::build(ctx)).await?;
    queue.register(crate::workers::product_sync::Worker::build(ctx)).await?;
    queue.register(crate::workers::order_creation::OrderCreationWorker::build(ctx)).await?;
    queue.register(crate::workers::shipping_rate_sync::ShippingRateSyncWorker::build(ctx)).await?;
    Ok(())
}
```

- [x] **Step 4: Compilar para verificar**

```bash
cd /home/andy/Proyectos/odoo_loco_shop/rust_backend && cargo check 2>&1 | tail -5
```

---

### Task 3: Odoo — `_sync_to_shop()`, create/write overrides

**Files:**
- Modify: `odoo_custom_addons/odoo_rust_sync/models/shipping_rate.py`

**Steps:**

- [x] **Step 1: Sobrescribir `models/shipping_rate.py` con los métodos nuevos**

```python
from odoo import fields, models, api
import logging
import requests

_logger = logging.getLogger(__name__)


class ShippingRate(models.Model):
    _name = "shipping.rate"
    _description = "Shipping Rate by State Pair"

    origin_country_id = fields.Many2one("res.country", string="País origen", required=True)
    origin_state_id = fields.Many2one("res.country.state", string="Estado origen", required=True)
    dest_country_id = fields.Many2one("res.country", string="País destino", required=True)
    dest_state_id = fields.Many2one("res.country.state", string="Estado destino", required=True)
    amount = fields.Float(string="Costo de envío", required=True)

    _sql_constraints = [
        (
            "unique_route",
            "unique(origin_country_id, origin_state_id, dest_country_id, dest_state_id)",
            "Ya existe una tarifa para esta ruta",
        ),
    ]

    def _compute_display_name(self):
        for r in self:
            r.display_name = f"{r.origin_state_id.name} → {r.dest_state_id.name}: ${r.amount:.2f}"

    def _sync_to_shop(self):
        shop_url = self.env["ir.config_parameter"].sudo().get_param(
            "rust_shop_url", "http://localhost:5150"
        )
        token = self.env["ir.config_parameter"].sudo().get_param("rust_shop_token", "")
        if not token:
            _logger.warning("rust_shop_token no configurado, saltando sync de tarifas")
            return
        rates_data = [
            {
                "origin_country": r.origin_country_id.with_context(lang="en_US").name,
                "origin_state": r.origin_state_id.name,
                "dest_country": r.dest_country_id.with_context(lang="en_US").name,
                "dest_state": r.dest_state_id.name,
                "amount": r.amount,
            }
            for r in self
        ]
        try:
            resp = requests.post(
                f"{shop_url}/api/shipping/rates/sync",
                headers={
                    "Authorization": f"Bearer {token}",
                    "Content-Type": "application/json",
                },
                json={"rates": rates_data},
                timeout=10,
            )
            if resp.status_code not in (200, 202):
                _logger.error(
                    "Error sync tarifas: HTTP %s - %s", resp.status_code, resp.text[:200]
                )
            else:
                _logger.info("Tarifas sincronizadas con la tienda (%d registros)", len(rates_data))
        except Exception as e:
            _logger.error("Error conectando con la tienda: %s", e)

    @api.model_create_multi
    def create(self, vals_list):
        records = super().create(vals_list)
        records._sync_to_shop()
        return records

    def write(self, vals):
        result = super().write(vals)
        if result:
            self._sync_to_shop()
        return result
```

---

### Task 4: Odoo — Server action

**Files:**
- Modify: `odoo_custom_addons/odoo_rust_sync/views/shipping_views.xml`

**Steps:**

- [x] **Step 1: Agregar server action antes del cierre `</data>`**

```xml
        <!-- Server action: sync selected rates to Rust shop -->
        <record id="action_sync_shipping_rates" model="ir.actions.server">
            <field name="name">Sincronizar con la tienda</field>
            <field name="model_id" ref="model_shipping_rate"/>
            <field name="binding_model_id" ref="model_shipping_rate"/>
            <field name="state">code</field>
            <field name="code">records._sync_to_shop()</field>
        </record>
```

---

### Task 5: Build global + test

**Steps:**

- [x] **Step 1: Build Rust final**

```bash
cd /home/andy/Proyectos/odoo_loco_shop/rust_backend && cargo build 2>&1 | tail -5
```

- [x] **Step 2: Verificar que el endpoint responde (backend arriba)**

```bash
# Primero configurar token en la DB (si no existe)
source ~/Entornos\ virtuales/odoo18/.venv/bin/activate && python3 -c "
import psycopg2
conn = psycopg2.connect('postgresql://postgres:postgres@localhost:5432/odoo_shop_development')
cur = conn.cursor()
cur.execute(\"SELECT value FROM configs WHERE key = 'webhook_token'\")
row = cur.fetchone()
print('Token actual:', row[0] if row else 'no configurado')
cur.close()
conn.close()
"

# Probar endpoint con token
curl -s -X POST "http://localhost:5150/api/shipping/rates/sync" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"rates":[{"origin_country":"Cuba","origin_state":"Havana","dest_country":"Cuba","dest_state":"Matanzas","amount":100.0}]}'
echo ""

# Verificar que se guardó en la DB
source ~/Entornos\ virtuales/odoo18/.venv/bin/activate && python3 -c "
import psycopg2
conn = psycopg2.connect('postgresql://postgres:postgres@localhost:5432/odoo_shop_development')
cur = conn.cursor()
cur.execute('SELECT * FROM shipping_rates')
for row in cur.fetchall():
    print(row)
cur.close()
conn.close()
"
```

- [x] **Step 3: Verificar Odoo — actualizar módulo**

```bash
# Actualizar módulo odoo_rust_sync desde Odoo para que tome los cambios XML
```

---

## Post-implementation note (2026-07-14)

### Critical: `--server-and-worker` required

Loco `start` por defecto corre en `ServerOnly` — **no spawnée worker threads**. Todos los `BackgroundWorker::perform_later()` encolan jobs en Redis correctamente, pero nadie los procesa.

**Síntoma:** endpoint responde `{"status":"accepted"}` (HTTP 200), el job aparece en `queue:default` de Redis con `status: queued`, pero nunca se procesa.

**Solución:** Arrancar con `--server-and-worker`:
```bash
cd rust_backend && setsid -f bash -c 'exec target/debug/odoo_shop-cli start --server-and-worker' &>/tmp/rust_shop.log
```

Esto afecta a **todos** los workers del proyecto: WebhookWorker, ProductSync, OrderCreationWorker, ShippingRateSyncWorker.

### Workers verificados

- `POST /api/shipping/rates/sync` → `{"status":"accepted"}` → worker upserta en `shipping_rates` por unique key `(origin_country, origin_state, dest_country, dest_state)`
- Shipping estimate con warehouse Cuba/Havana → destino Cuba/Matanzas devuelve `"shipping_cost":"250"` (según tarifa actual)
- Healthcheck 17/17 endpoints respondiendo
```
