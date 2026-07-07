# Shipping Cost por Zona/Región — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar costo de envío dinámico basado en ubicación del almacén del producto (Odoo) y dirección del usuario, con consolidación en checkout.

**Architecture:** Configuración vía modelos custom en Odoo → sincronización a PostgreSQL vía Rust worker → cálculo en checkout (Rust). Usa el mismo patrón que `product_sync.rs`.

**Tech Stack:** Odoo 18.0 (Python), Rust (SeaORM, Loco.rs), PostgreSQL, Tera templates, Vue 3

## Global Constraints

- `i32` para IDs de Odoo, `Uuid` para IDs internos
- Nombres de país/estado guardados como `String` en Rust (no dependemos de FK de Odoo)
- prefijo `m20260707_` para nuevas migraciones
- Usar `#[debug_handler]` en controllers nuevos
- Usar `use loco_rs::prelude::*` para imports comunes
- Usar `use sea_orm::ActiveValue::Set` para inserts/updates
- `rsv_cart_session` para cookie de carrito invitado
- No crear documentación (*.md) ni README a menos que se pida explícitamente
- Preferir `edit` sobre `write` para archivos existentes
- No hacer commit a menos que se pida explícitamente

---

### Task 1: Migraciones Rust (3 migrations)

**Files:**
- Create: `rust_backend/migration/src/m20260707_000001_add_warehouse_to_products.rs`
- Create: `rust_backend/migration/src/m20260707_000002_create_shipping_rates.rs`
- Create: `rust_backend/migration/src/m20260707_000003_add_country_state_to_users.rs`
- Modify: `rust_backend/migration/src/lib.rs`

**Interfaces:**
- Consumes: existing migration patterns (`m20260622_000002_create_orders.rs`, `m20260703_000001_add_profile_fields_to_users.rs`)
- Produces: 3 nuevas migraciones registradas en lib.rs

- [ ] **Step 1: Crear `m20260707_000001_add_warehouse_to_products.rs`**

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseCountry).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseState).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseLat).double().null())
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseLng).double().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::WarehouseCountry)
                    .drop_column(Products::WarehouseState)
                    .drop_column(Products::WarehouseLat)
                    .drop_column(Products::WarehouseLng)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Products {
    Table,
    WarehouseCountry,
    WarehouseState,
    WarehouseLat,
    WarehouseLng,
}
```

- [ ] **Step 2: Crear `m20260707_000002_create_shipping_rates.rs`**

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShippingRates::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ShippingRates::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(ShippingRates::OriginCountry).string().not_null())
                    .col(ColumnDef::new(ShippingRates::OriginState).string().not_null())
                    .col(ColumnDef::new(ShippingRates::DestCountry).string().not_null())
                    .col(ColumnDef::new(ShippingRates::DestState).string().not_null())
                    .col(ColumnDef::new(ShippingRates::Amount).decimal(12, 2).not_null())
                    .col(ColumnDef::new(ShippingRates::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(ShippingRates::UpdatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ShippingRates::Table)
                    .unique()
                    .col(ShippingRates::OriginCountry)
                    .col(ShippingRates::OriginState)
                    .col(ShippingRates::DestCountry)
                    .col(ShippingRates::DestState)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ShippingRates::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum ShippingRates {
    Table,
    Id,
    OriginCountry,
    OriginState,
    DestCountry,
    DestState,
    Amount,
    CreatedAt,
    UpdatedAt,
}
```

- [ ] **Step 3: Crear `m20260707_000003_add_country_state_to_users.rs`**

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column_if_not_exists(ColumnDef::new(Users::Country).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Users::State).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Country)
                    .drop_column(Users::State)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Country,
    State,
}
```

- [ ] **Step 4: Registrar migraciones en `lib.rs`**

En `rust_backend/migration/src/lib.rs`, agregar al array `migrations![]`:

```rust
mod m20260707_000001_add_warehouse_to_products;
mod m20260707_000002_create_shipping_rates;
mod m20260707_000003_add_country_state_to_users;
```

Y en la lista:

```rust
m20260707_000001_add_warehouse_to_products::Migration,
m20260707_000002_create_shipping_rates::Migration,
m20260707_000003_add_country_state_to_users::Migration,
```

- [ ] **Step 5: Verificar que compilan**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -30
```

---

### Task 2: Actualizar Entities SeaORM

**Files:**
- Modify: `rust_backend/src/models/_entities/products.rs`
- Create: `rust_backend/src/models/_entities/shipping_rates.rs`
- Modify: `rust_backend/src/models/_entities/mod.rs`
- Modify: `rust_backend/src/models/_entities/users.rs`

**Interfaces:**
- Consumes: migrations de Task 1
- Produces: `Products` entity con campos warehouse, `ShippingRates` entity, `Users` entity con country/state

- [ ] **Step 1: Agregar campos warehouse a `_entities/products.rs`**

Agregar después del campo `tax_percent`:

```rust
    pub warehouse_country: Option<String>,
    pub warehouse_state: Option<String>,
    pub warehouse_lat: Option<f64>,
    pub warehouse_lng: Option<f64>,
```

En `Column` enum, agregar:

```rust
    WarehouseCountry,
    WarehouseState,
    WarehouseLat,
    WarehouseLng,
```

En la implementación de `Column` trait:

```rust
            Self::WarehouseCountry => ColumnType::String(None),
            Self::WarehouseState => ColumnType::String(None),
            Self::WarehouseLat => ColumnType::Double(None),
            Self::WarehouseLng => ColumnType::Double(None),
```

- [ ] **Step 2: Crear `_entities/shipping_rates.rs`**

```rust
//! SeaORM Entity. Generated by sea-orm-codegen

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "shipping_rates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub origin_country: String,
    pub origin_state: String,
    pub dest_country: String,
    pub dest_state: String,
    pub amount: Decimal,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

- [ ] **Step 3: Agregar country/state a `_entities/users.rs`**

Agregar después del campo `zip`:

```rust
    pub country: Option<String>,
    pub state: Option<String>,
```

En `Column` enum:

```rust
    Country,
    State,
```

En implementación de `Column` trait:

```rust
            Self::Country => ColumnType::String(None),
            Self::State => ColumnType::String(None),
```

- [ ] **Step 4: Registrar `shipping_rates` en `_entities/mod.rs`**

```rust
pub mod shipping_rates;
```

- [ ] **Step 5: Verificar compilación**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -20
```

---

### Task 3: Modelo Rust ShippingRates (module)

**Files:**
- Create: `rust_backend/src/models/shipping_rates.rs`
- Modify: `rust_backend/src/models/mod.rs`

**Interfaces:**
- Consumes: `_entities/shipping_rates.rs` de Task 2
- Produces: `shipping_rates::Model`, función `find_rate()` para consultar tarifa

- [ ] **Step 1: Crear `src/models/shipping_rates.rs`**

```rust
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::models::_entities::shipping_rates;

pub async fn find_rate(
    db: &DatabaseConnection,
    origin_country: &str,
    origin_state: &str,
    dest_country: &str,
    dest_state: &str,
) -> Result<Option<Decimal>, DbErr> {
    shipping_rates::Entity::find()
        .filter(shipping_rates::Column::OriginCountry.eq(origin_country))
        .filter(shipping_rates::Column::OriginState.eq(origin_state))
        .filter(shipping_rates::Column::DestCountry.eq(dest_country))
        .filter(shipping_rates::Column::DestState.eq(dest_state))
        .one(db)
        .await
        .map(|r| r.map(|m| m.amount))
}

pub async fn find_rate_by_country(
    db: &DatabaseConnection,
    origin_country: &str,
    dest_country: &str,
    dest_state: &str,
) -> Result<Option<Decimal>, DbErr> {
    shipping_rates::Entity::find()
        .filter(shipping_rates::Column::OriginCountry.eq(origin_country))
        .filter(shipping_rates::Column::DestCountry.eq(dest_country))
        .filter(shipping_rates::Column::DestState.eq(dest_state))
        .one(db)
        .await
        .map(|r| r.map(|m| m.amount))
}

pub async fn replace_all(
    db: &DatabaseConnection,
    rates: Vec<shipping_rates::ActiveModel>,
) -> Result<(), DbErr> {
    shipping_rates::Entity::delete_many().exec(db).await?;
    for rate in rates {
        rate.insert(db).await?;
    }
    Ok(())
}
```

- [ ] **Step 2: Registrar en `src/models/mod.rs`**

```rust
pub mod shipping_rates;
```

- [ ] **Step 3: Compilar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -20
```

---

### Task 4: Modelos en Odoo

**Files:**
- Create: `odoo_custom_addons/odoo_rust_sync/models/shipping_rate.py`
- Modify: `odoo_custom_addons/odoo_rust_sync/models/product_template.py` (o crear si no existe)
- Modify: `odoo_custom_addons/odoo_rust_sync/models/__init__.py`
- Modify: `odoo_custom_addons/odoo_rust_sync/__manifest__.py`

**Interfaces:**
- Consumes: `res.country`, `res.country.state` de Odoo core
- Produces: `shipping.rate` modelo, warehouse fields en `product.template`

- [ ] **Step 1: Crear `models/shipping_rate.py`**

```python
from odoo import fields, models


class ShippingRate(models.Model):
    _name = "shipping.rate"
    _description = "Shipping Rate by State Pair"
    _rec_name = "display_name"

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

    def display_name(self):
        for r in self:
            r.display_name = f"{r.origin_state_id.name} → {r.dest_state_id.name}: ${r.amount:.2f}"
```

- [ ] **Step 2: Modificar `models/product_template.py`**

Si no existe, crearlo:

```python
from odoo import fields, models


class ProductTemplate(models.Model):
    _inherit = "product.template"

    warehouse_country_id = fields.Many2one("res.country", string="País del almacén")
    warehouse_state_id = fields.Many2one("res.country.state", string="Estado del almacén")
    warehouse_latitude = fields.Float(string="Latitud del almacén", digits=(9, 6))
    warehouse_longitude = fields.Float(string="Longitud del almacén", digits=(9, 6))
```

- [ ] **Step 3: Actualizar `models/__init__.py`**

```python
from . import product_template
from . import shipping_rate
```

- [ ] **Step 4: Verificar `__manifest__.py`**

Asegurar que `depends` incluya `stock` (por el modelo `product.template` ya heredado). Si no existe dependencia a `stock`, no es necesario — `product.template` está en `sale`.

---

### Task 5: Worker de Sincronización (shipping_sync.rs)

**Files:**
- Create: `rust_backend/src/workers/shipping_sync.rs`
- Modify: `rust_backend/src/workers/mod.rs`
- Modify: `rust_backend/src/app.rs` (registrar worker)

**Interfaces:**
- Consumes: `odoo_db_uri` de configs (misma que `product_sync.rs`)
- Consumes: `shipping_rates::replace_all()`, entidad `products`
- Produces: tarea CLI `sync-shipping`

- [ ] **Step 1: Crear `src/workers/shipping_sync.rs`**

```rust
use loco_rs::prelude::*;
use sea_orm::{ConnectOptions, Database, EntityTrait, QueryFilter, ColumnTrait, Set};
use crate::models::_entities::{products, shipping_rates};
use crate::models::shipping_rates as shipping_model;
use sea_orm::ActiveValue;

pub struct ShippingSyncWorker;

#[async_trait]
impl Task for ShippingSyncWorker {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "sync-shipping".to_string(),
            detail: "Sync warehouse locations and shipping rates from Odoo".to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, _vars: &TaskVars) -> Result<()> {
        let db_uri = crate::models::config_cache::get_cached_config(app_context, "odoo_db_uri")
            .await
            .unwrap_or_else(|| "postgres://odoo:postgres@localhost:5432/odoo_prod".to_string());

        let odoo_db = Database::connect(
            ConnectOptions::new(db_uri).to_owned(),
        )
        .await
        .map_err(|e| tracing::error!("Failed to connect to Odoo DB: {}", e))
        .unwrap();

        // Sync warehouse locations
        let warehouses = sqlx::query_as::<_, (i32, Option<String>, Option<String>, Option<f64>, Option<f64>)>(
            "SELECT pt.id, rc.name AS country, rcs.name AS state, pt.warehouse_latitude, pt.warehouse_longitude
             FROM product_template pt
             LEFT JOIN res_country rc ON pt.warehouse_country_id = rc.id
             LEFT JOIN res_country_state rcs ON pt.warehouse_state_id = rcs.id
             WHERE pt.sale_ok = true AND pt.warehouse_country_id IS NOT NULL"
        )
        .fetch_all(&odoo_db)
        .await
        .unwrap_or_default();

        for (odoo_id, country, state, lat, lng) in &warehouses {
            if let Some(product) = products::Entity::find()
                .filter(products::Column::OdooId.eq(*odoo_id))
                .one(&app_context.db)
                .await?
            {
                let mut active: products::ActiveModel = product.into();
                active.warehouse_country = Set(country.clone());
                active.warehouse_state = Set(state.clone());
                active.warehouse_lat = Set(*lat);
                active.warehouse_lng = Set(*lng);
                active.update(&app_context.db).await?;
            }
        }

        tracing::info!("Synced {} warehouse locations", warehouses.len());

        // Sync shipping rates
        let rates = sqlx::query_as::<_, (String, String, String, String, f64)>(
            "SELECT rc_o.name AS origin_country, rcs_o.name AS origin_state,
                    rc_d.name AS dest_country, rcs_d.name AS dest_state,
                    sr.amount
             FROM shipping_rate sr
             JOIN res_country rc_o ON sr.origin_country_id = rc_o.id
             JOIN res_country_state rcs_o ON sr.origin_state_id = rcs_o.id
             JOIN res_country rc_d ON sr.dest_country_id = rc_d.id
             JOIN res_country_state rcs_d ON sr.dest_state_id = rcs_d.id"
        )
        .fetch_all(&odoo_db)
        .await
        .unwrap_or_default();

        let models: Vec<shipping_rates::ActiveModel> = rates
            .into_iter()
            .map(|(oc, os, dc, ds, amt)| shipping_rates::ActiveModel {
                origin_country: ActiveValue::Set(oc),
                origin_state: ActiveValue::Set(os),
                dest_country: ActiveValue::Set(dc),
                dest_state: ActiveValue::Set(ds),
                amount: ActiveValue::Set(Decimal::try_from(amt).unwrap_or(Decimal::ZERO)),
                ..Default::default()
            })
            .collect();

        if !models.is_empty() {
            shipping_model::replace_all(&app_context.db, models).await?;
        }

        tracing::info!("Synced {} shipping rates", models.len());
        Ok(())
    }
}
```

- [ ] **Step 2: Registrar en `src/workers/mod.rs`**

```rust
pub mod shipping_sync;
```

```rust
_ => shipping_sync::ShippingSyncWorker,
```

En la función `fn tasks()`:

```rust
("sync-shipping".to_string(), Box::new(shipping_sync::ShippingSyncWorker)),
```

- [ ] **Step 3: Registrar en `app.rs`**

En `App::tasks`, agregar la tarea `sync-shipping`.

```rust
tasks::register("sync-shipping", &shipping_sync_task),
```

Donde `shipping_sync_task` se importa de `workers::shipping_sync::ShippingSyncWorker`.

(Ver el patrón exacto de `app.rs` para workers existentes como `product_sync`)

- [ ] **Step 4: Compilar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -30
```

---

### Task 6: Perfil de Usuario — country/state

**Files:**
- Modify: `rust_backend/src/controllers/views.rs` (estructura ProfileForm, update_profile)
- Modify: `rust_backend/assets/views/auth/profile.html`

**Interfaces:**
- Consumes: `users::Column::Country`, `users::Column::State`
- Produces: formulario de perfil con campos país/estado

- [ ] **Step 1: Agregar country/state a `ProfileForm` en `views.rs`**

```rust
pub country: Option<String>,
pub state: Option<String>,
```

- [ ] **Step 2: En `update_profile` (views.rs), guardar country/state**

```rust
if let Some(country) = &params.country {
    if !country.is_empty() {
        user.as_mut().country = Set(Some(country.clone()));
    }
}
if let Some(state) = &params.state {
    if !state.is_empty() {
        user.as_mut().state = Set(Some(state.clone()));
    }
}
```

- [ ] **Step 3: En `profile.html`, agregar inputs después de zip**

```html
<div class="mb-4">
    <label class="block text-sm font-medium mb-1">País</label>
    <input type="text" name="country" value="{{ current_user.country or '' }}"
           class="w-full border rounded px-3 py-2">
</div>
<div class="mb-4">
    <label class="block text-sm font-medium mb-1">Estado / Provincia</label>
    <input type="text" name="state" value="{{ current_user.state or '' }}"
           class="w-full border rounded px-3 py-2">
</div>
```

- [ ] **Step 4: Compilar y verificar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -20
```

---

### Task 7: Cálculo de Shipping en Checkout

**Files:**
- Modify: `rust_backend/src/controllers/checkout.rs`
- Modify: `rust_backend/assets/views/shop/checkout.html`
- Modify: `rust_backend/assets/static/js/checkout.js`

**Interfaces:**
- Consumes: `shipping_rates::find_rate()`, `products::Column::Warehouse*`
- Produces: `shipping_cost` en la orden, display en checkout

- [ ] **Step 1: En `checkout_page` (GET), pasar `shipping_rates` como JSON contexto**

Después de cargar `user_data`, cargar las shipping_rates en un map para cálculo client-side:

```rust
// Cargar rates para cálculo rápido
let rates = shipping_rates::Entity::find().all(&ctx.db).await?;
let rates_json = serde_json::to_value(&rates).unwrap_or_default();
```

Pasar al template:

```rust
json!({
    "items": items,
    "total": total,
    "current_user": user_view,
    "user_data": user_data,
    "payment_methods": payment_methods,
    "shipping_rates": rates_json,
})
```

- [ ] **Step 2: Función de cálculo de shipping en checkout_page/submit_checkout**

En `checkout.rs`, agregar función helper:

```rust
async fn calc_shipping(
    db: &DatabaseConnection,
    items: &[CartWithProduct],
    country: &str,
    state: &str,
) -> Result<(Decimal, String), DbErr> {
    // Agrupar productos por warehouse único
    let mut origins: Vec<(&str, &str)> = Vec::new();
    for item in items {
        if let (Some(c), Some(s)) = (&item.product.warehouse_country, &item.product.warehouse_state) {
            if !origins.iter().any(|(oc, os)| oc == c && os == s) {
                origins.push((c, s));
            }
        }
    }

    if origins.is_empty() {
        return Ok((Decimal::ZERO, "Sin origen definido".to_string()));
    }

    let mut max_rate = Decimal::ZERO;
    let mut origin_desc = String::new();

    for (oc, os) in &origins {
        // Intentar rate exacto
        let rate = shipping_rates::find_rate(db, oc, os, country, state).await?
            .or_else(|| {
                // Fallback: mismo país, cualquier estado origen
                futures::executor::block_on(
                    shipping_rates::find_rate_by_country(db, oc, country, state)
                ).ok().flatten()
            })
            .unwrap_or(Decimal::ZERO);

        if rate > max_rate {
            max_rate = rate;
            origin_desc = format!("{}, {}", os, oc);
        }
    }

    Ok((max_rate, origin_desc))
}
```

- [ ] **Step 3: En `submit_checkout`, usar `calc_shipping`**

Después de validar items y antes de crear la orden:

```rust
let dest_country = customer_country.as_deref().unwrap_or("");
let dest_state = customer_state.as_deref().unwrap_or("");

let (shipping_cost, shipping_origin) = calc_shipping(
    &ctx.db,
    &items,
    dest_country,
    dest_state,
)
.await?;

// Verificar tarifa local (mismo país y estado para todos los productos)
let all_local = items.iter().all(|item| {
    item.product.warehouse_country.as_deref() == Some(dest_country)
        && item.product.warehouse_state.as_deref() == Some(dest_state)
});
let shipping_cost = if all_local {
    let local_rate = crate::models::config_cache::get_cached_config(&ctx, "shipping_local_rate")
        .await
        .unwrap_or_else(|| "0.00".to_string());
    Decimal::try_from(local_rate.parse::<f64>().unwrap_or(0.0)).unwrap_or(Decimal::ZERO)
} else {
    shipping_cost
};

// Agregar shipping al total
let total = total + shipping_cost;
```

En la creación de la orden:

```rust
shipping_cost: Set(shipping_cost),
```

- [ ] **Step 4: En `checkout.html`, mostrar shipping**

Después de la línea de subtotal, agregar:

```html
<div class="flex justify-between py-2">
    <span>Envío <small v-if="shippingOrigin">(desde [[ shippingOrigin ]])</small></span>
    <span>[[ shippingCost ? '$' + shippingCost.toFixed(2) : 'A calcular' ]]</span>
</div>
```

Y en el computed de Vue, agregar `shippingCost` y `shippingOrigin`.

- [ ] **Step 5: En `checkout.js`, enviar country/state en POST**

Agregar al payload de `submitOrder`:

```javascript
customer.country = this.customer.country || '';
customer.state = this.customer.state || '';
```

- [ ] **Step 6: Compilar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -20
```

---

### Task 8: Endpoint `/api/shipping/estimate`

**Files:**
- Create or Modify: `rust_backend/src/controllers/shipping.rs`
- Modify: `rust_backend/src/controllers/mod.rs`

**Interfaces:**
- Consumes: `calc_shipping()` de Task 7
- Produces: GET endpoint para estimación de costo

- [ ] **Step 1: Crear `src/controllers/shipping.rs`**

```rust
use axum::extract::Query;
use loco_rs::prelude::*;
use serde::Deserialize;
use crate::models::_entities::products;

#[derive(Debug, Deserialize)]
pub struct ShippingEstimateParams {
    pub product_ids: String, // comma-separated
    pub country: String,
    pub state: String,
}

#[debug_handler]
pub async fn estimate(
    State(ctx): State<AppContext>,
    Query(params): Query<ShippingEstimateParams>,
) -> impl IntoResponse {
    let ids: Vec<i32> = params.product_ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let items = products::Entity::find()
        .filter(products::Column::Id.is_in(ids))
        .all(&ctx.db)
        .await
        .unwrap_or_default();

    // Reuse calc_shipping logic — convert to CartWithProduct-like structure
    // or extract the shipping calc to a shared function
    let (shipping_cost, origin_summary) = crate::controllers::checkout::calc_shipping(
        &ctx.db,
        &items,
        &params.country,
        &params.state,
    )
    .await
    .unwrap_or((Decimal::ZERO, String::new()));

    json(serde_json::json!({
        "shipping_cost": shipping_cost,
        "origin_summary": origin_summary,
    }))
}

pub fn routes() -> Routes {
    Routes::new().prefix("api/shipping").add("/estimate", get(estimate))
}
```

- [ ] **Step 2: Registrar ruta en `shipping.rs` y `mod.rs`**

En `controllers/mod.rs`:

```rust
pub mod shipping;
```

En `app.rs` (o donde se registran rutas), agregar:

```rust
controllers::shipping::routes()
```

- [ ] **Step 3: Compilar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -20
```

---

### Task 9: UI — mostrar shipping en órdenes

**Files:**
- Modify: `rust_backend/src/controllers/views.rs` (orders_page pasa shipping_cost)
- Modify: `rust_backend/assets/views/shop/orders.html`

- [ ] **Step 1: En `orders_page`, pasar shipping_cost por orden**

El `orders::Model` ya debe tener `shipping_cost` (agregado en Task 7). En el template se accede como `order.shipping_cost`.

- [ ] **Step 2: En `orders.html`, agregar columna shipping**

Después del total:

```html
<td class="px-4 py-2">${{ order.shipping_cost | round(precision=2) }}</td>
```

Y en el header:

```html
<th class="px-4 py-2">Envío</th>
```

- [ ] **Step 3: Compilar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -20
```

---

### Task 10: Config keys y UI admin para shipping

**Files:**
- Modify: `rust_backend/src/controllers/views.rs` (handle_config_update)

- [ ] **Step 1: Agregar defaults para shipping en configs**

En la UI de config (`config/ui.html` o `handle_config_update`) no es estrictamente necesario si los defaults se manejan en código. Pero agregar las keys documentadas:

```rust
// En el handler que obtiene configs para la UI:
("shipping_default_rate", "10.00"),
("shipping_local_rate", "0.00"),
```

- [ ] **Step 2: Compilar**

```bash
cd rust_backend
cargo build --all-features 2>&1 | head -10
```
