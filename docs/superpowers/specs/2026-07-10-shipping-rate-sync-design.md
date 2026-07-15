# Shipping Rate Sync — Design Spec

## Problem

Shipping rates configuradas en el modelo Odoo `shipping.rate` deben sincronizarse en
tiempo real con el backend Rust/Loco para que el checkout calcule costos de envío
actualizados. Hoy solo existe un task CLI (`sync-shipping`) que consulta la DB de Odoo
directamente — no hay sincronización automática al crear/modificar tarifas.

## Scope

Añadir un endpoint HTTP + background worker en Rust que reciba tarifas y las upsert, y
hooks Odoo (create/write + server action) que disparen el envío.

---

## Rust/Loco

### Endpoint

```
POST /api/shipping/rates/sync
Authorization: Bearer <webhook_token>
Content-Type: application/json

{
  "rates": [
    {
      "origin_country": "Cuba",
      "origin_state": "Havana",
      "dest_country": "Cuba",
      "dest_state": "Matanzas",
      "amount": 100.0
    }
  ]
}
```

- Autenticación: mismo `webhook_token` de la tabla `configs` que usan los demás
  webhooks (reutiliza `AuthToken` extractor existente en `middleware/auth_extractor.rs`).
- Respuesta: `202 {"status": "accepted"}` inmediata.
- Validación: array no vacío, cada rate con 5 campos requeridos, amount ≥ 0.
- Sin cambios en la ruta de `estimate` existente.

### Worker: `ShippingRateSyncWorker`

- Archivo nuevo `src/workers/shipping_rate_sync.rs`.
- Implementa `BackgroundWorker<ShippingRateSyncArgs>`.
- `ShippingRateSyncArgs { rates: Vec<RatePayload> }`.
- `RatePayload { origin_country, origin_state, dest_country, dest_state, amount }`.
- Lógica: por cada rate, busca por unique key `(origin_country, origin_state,
  dest_country, dest_state)`:
  - Si existe → actualiza `amount` y `updated_at`.
  - Si no existe → inserta.
- Loggea con `tracing::info!()` el resultado (cantidad upserted).
- Registrado en `app.rs` `connect_workers()`.

### Archivos a modificar/crear

| Archivo | Cambio |
|---------|--------|
| `src/workers/mod.rs` | Agregar `pub mod shipping_rate_sync;` |
| `src/workers/shipping_rate_sync.rs` | Crear — worker + tipos |
| `src/controllers/shipping.rs` | Agregar handler `sync_rates` + ruta POST `/rates/sync` |
| `src/app.rs` | Agregar `queue.register(...)` en `connect_workers` |
| `src/models/shipping_rates.rs` | Agregar función `upsert_rate` |

---

## Odoo

### Modelo: `shipping.rate`

Método nuevo `_sync_to_shop()` en el modelo `shipping.rate`:

```python
def _sync_to_shop(self):
    shop_url = self.env['ir.config_parameter'].sudo().get_param('rust_shop_url')
    token = self.env['ir.config_parameter'].sudo().get_param('rust_shop_token')
    rates = [{
        'origin_country': r.origin_country_id.with_context(lang='en_US').name,
        'origin_state': r.origin_state_id.name,
        'dest_country': r.dest_country_id.with_context(lang='en_US').name,
        'dest_state': r.dest_state_id.name,
        'amount': r.amount,
    } for r in self]
    try:
        requests.post(f"{shop_url}/api/shipping/rates/sync",
                      headers={'Authorization': f'Bearer {token}'},
                      json={'rates': rates}, timeout=10)
    except Exception:
        _logger.error(...)  # no bloquea la operación
```

### Hooks automáticos

- `create()`: override con `@api.model_create_multi`. Llama a `_sync_to_shop()` post
  creación con los registros recién creados.
- `write()`: override. Llama a `_sync_to_shop()` post actualización si el write fue
  exitoso.
- Ambos envuelven el HTTP call en try/except para no bloquear la operación de Odoo.

### Server action manual

XML record en `views/shipping_views.xml`:

```xml
<record id="action_sync_shipping_rates" model="ir.actions.server">
  <field name="name">Sincronizar con la tienda</field>
  <field name="model_id" ref="model_shipping_rate"/>
  <field name="binding_model_id" ref="model_shipping_rate"/>
  <field name="state">code</field>
  <field name="code">records._sync_to_shop()</field>
</record>
```

Aparece en el menú Action de la vista lista de `shipping.rate`.

### Configuración

Parámetros del sistema (`ir.config_parameter`) — se crean automáticamente con
valor por defecto la primera vez que se usan:

| Key | Default | Descripción |
|-----|---------|-------------|
| `rust_shop_url` | `http://localhost:5150` | URL base del backend Rust |
| `rust_shop_token` | `""` | Bearer token (debe coincidir con `webhook_token`) |

### Archivos a modificar/crear

| Archivo | Cambio |
|---------|--------|
| `models/shipping_rate.py` | Agregar `_sync_to_shop()`, overrides `create`/`write`, import `requests` |
| `views/shipping_views.xml` | Agregar server action record |

---

## Flujo completo

```
Odoo: crear/editar tarifa en UI
  → create() / write() override
  → _sync_to_shop() envía HTTP POST
  → Rust endpoint POST /api/shipping/rates/sync
  → AuthToken valida Bearer token
  → push a Redis queue (ShippingRateSyncWorker)
  → responde 202
  → worker upserta en shipping_rates
  → checkout ahora usa la tarifa actualizada

Odoo: Action > "Sincronizar con la tienda" (seleccionados)
  → mismo flujo desde el server action
```

## Edge cases

- **Worker falla** (Redis caído): endpoint responde error 503. Odoo loggea error, no
  bloquea al usuario.
- **Token incorrecto**: endpoint responde 401, Odoo loggea advertencia.
- **Rate duplicado**: el unique index upsert lo maneja (actualiza amount).
- **Create multi**: `@api.model_create_multi` llama `_sync_to_shop` una vez con todos
  los records creados, no N requests individuales.
- **Write batch**: Odoo llama `write(vals)` con `self` conteniendo todos los registros
  a actualizar. `_sync_to_shop` envía todos en un solo request.
