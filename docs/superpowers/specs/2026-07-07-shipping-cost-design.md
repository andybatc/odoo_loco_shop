# Shipping Cost por Zona/Región

## Resumen

Costo de envío dinámico calculado en checkout según la ubicación del almacén del producto (configurado en Odoo) y la dirección del usuario. Configuración vía Odoo, lógica de cálculo en Rust, consolidación de múltiples orígenes en un solo envío.

## Stack

- **Origen de datos**: Odoo 18.0 (modelos custom en `odoo_rust_sync`)
- **Sincronización**: Rust worker (nuevo `shipping_sync.rs`, patrón idéntico a `product_sync.rs`)
- **Cálculo**: Rust (`checkout.rs` en `submit_checkout`)
- **Almacenamiento**: PostgreSQL vía SeaORM (tablas `products` ampliada y `shipping_rates` nueva)

## Modelos en Odoo

### Product — campos de almacén

En `product.template`, agregar:

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `warehouse_country_id` | `Many2one(res.country)` | País donde está almacenado |
| `warehouse_state_id` | `Many2one(res.country.state)` | Estado/provincia del almacén |
| `warehouse_latitude` | `Float` | Coordenada latitud (decimal) |
| `warehouse_longitude` | `Float` | Coordenada longitud (decimal) |

### ShippingRate — modelo nuevo

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `origin_country_id` | `Many2one(res.country)` | País origen |
| `origin_state_id` | `Many2one(res.country.state)` | Estado origen |
| `dest_country_id` | `Many2one(res.country)` | País destino |
| `dest_state_id` | `Many2one(res.country.state)` | Estado destino |
| `amount` | `Float` | Costo de envío |

Restricción: unique(origin_country_id, origin_state_id, dest_country_id, dest_state_id).
El admin configura pares estado-origen → estado-destino con su tarifa.

## Migraciones en Rust

### `m20260707_000001_add_warehouse_to_products`

```sql
ALTER TABLE products ADD COLUMN warehouse_country VARCHAR(255);
ALTER TABLE products ADD COLUMN warehouse_state VARCHAR(255);
ALTER TABLE products ADD COLUMN warehouse_lat DOUBLE PRECISION;
ALTER TABLE products ADD COLUMN warehouse_lng DOUBLE PRECISION;
```

### `m20260707_000002_create_shipping_rates`

```sql
CREATE TABLE shipping_rates (
    id SERIAL PRIMARY KEY,
    origin_country VARCHAR(255) NOT NULL,
    origin_state VARCHAR(255) NOT NULL,
    dest_country VARCHAR(255) NOT NULL,
    dest_state VARCHAR(255) NOT NULL,
    amount DECIMAL(12,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(origin_country, origin_state, dest_country, dest_state)
);
```

### `m20260707_000003_add_country_state_to_users`

```sql
ALTER TABLE users ADD COLUMN country VARCHAR(255);
ALTER TABLE users ADD COLUMN state VARCHAR(255);
```

### Entities

- `_entities/products.rs`: agregar `warehouse_country: Option<String>`, `warehouse_state: Option<String>`, `warehouse_lat: Option<f64>`, `warehouse_lng: Option<f64>`
- Nuevo `_entities/shipping_rates.rs`: modelo estándar SeaORM con todos los campos
- `_entities/users.rs`: agregar `country: Option<String>`, `state: Option<String>`

## Sincronización

### Nuevo worker: `shipping_sync.rs`

Misma conexión a Odoo DB que `product_sync.rs` (usa `odoo_db_uri` de configs).

**Query 1 — Warehouses:**

```sql
SELECT pt.id, rc.name AS country, rcs.name AS state, pt.warehouse_latitude, pt.warehouse_longitude
FROM product_template pt
LEFT JOIN res_country rc ON pt.warehouse_country_id = rc.id
LEFT JOIN res_country_state rcs ON pt.warehouse_state_id = rcs.id
WHERE pt.sale_ok = true AND pt.warehouse_country_id IS NOT NULL
```

Actualiza `products.warehouse_country`, `products.warehouse_state`, `products.warehouse_lat`, `products.warehouse_lng` haciendo match por `odoo_id`.

**Query 2 — Rates:**

```sql
SELECT rc_o.name AS origin_country, rcs_o.name AS origin_state,
       rc_d.name AS dest_country, rcs_d.name AS dest_state,
       sr.amount
FROM shipping_rate sr
JOIN res_country rc_o ON sr.origin_country_id = rc_o.id
JOIN res_country_state rcs_o ON sr.origin_state_id = rcs_o.id
JOIN res_country rc_d ON sr.dest_country_id = rc_d.id
JOIN res_country_state rcs_d ON sr.dest_state_id = rcs_d.id
```

Reemplaza toda la tabla `shipping_rates` (truncate + insert) ya que es pequeña.

**Trigger**: el worker se ejecuta después de `product_sync` (mismo schedule, o como paso final dentro de product_sync).

## Cálculo en checkout

### Lógica de `submit_checkout`

```
1. Obtener items del carrito con productos (incluye warehouse_country, warehouse_state)
2. Obtener dirección del usuario (de user_data en request o del perfil)
3. Agrupar productos únicos por (warehouse_country, warehouse_state)
4. Para cada grupo:
   a. Buscar shipping_rate WHERE origin = (warehouse_country, warehouse_state)
        AND dest = (user_country, user_state)
   b. Si no existe rate directo → buscar rate genérico para el país destino
      (origin_state = cualquier, dest_country = user_country)
   c. Si tampoco → usar tarifa_default de configs
5. shipping_cost = max rate entre todos los grupos (consolidación, un solo envío)
6. Si todos los productos están en mismo país+estado que el usuario → shipping_local_rate (0 o mínimo)
7. Agregar shipping_cost al total de la orden
8. Guardar en orders: shipping_cost

Inputs del POST /api/checkout:
{
  customer: { name, email, phone, street, city, zip, country, state },
  payment_method_id: int
}

Output:
{
  shipping_cost: Decimal,
  total: Decimal,
  redirect: "/order/success?ref=..."
}
```

### Tarifas por defecto

Dos keys nuevas en `configs`:

| Key | Default | Descripción |
|-----|---------|-------------|
| `shipping_default_rate` | `"0.00"` | Tarifa cuando no hay rate configurado para el par |
| `shipping_local_rate` | `"0.00"` | Tarifa cuando origen = destino |

## UI/UX

### Checkout

En `checkout.html`, se muestra línea de envío con el costo calculado:

```
Subtotal:     $100.00
Envío (Bs.As. → Córdoba):  $12.00
Total:        $112.00
```

Vue computed property `shippingCost` que se calcula desde `userData` (con country/state) y los items del carrito (con warehouse). Si el usuario cambia su dirección de envío, se recalcula vía llamada a `/api/shipping/estimate` (endpoint nuevo liviano que solo calcula el costo sin crear orden).

### Perfil

Dos nuevos campos en `auth/profile.html`: `country` (text), `state` (text). Se guardan en `update_profile`.

### Órdenes

`orders.html` muestra `shipping_cost` y destino (`customer_city`, `customer_state`).

## Endpoints

| Método | Ruta | Propósito |
|--------|------|-----------|
| `GET` | `/api/shipping/estimate` | Calcula shipping cost sin crear orden (para recálculo en checkout) |

Input: query params `product_ids[]`, `country`, `state`
Output: `{ shipping_cost, origin_summary, consolidated }`

## Orden de implementación

1. Migraciones Rust (3 migrations)
2. Modelos en Odoo (campos warehouse + shipping_rate)
3. Worker shipping_sync
4. Perfil: country + state en users y UI
5. Checkout: cálculo de shipping + display
6. Endpoint `/api/shipping/estimate`
7. UI de órdenes: mostrar shipping

## Notas

- Los nombres de país/estado se guardan como strings en Rust (no dependemos de IDs de Odoo)
- Consolidación: un solo envío, se cobra el rate más alto entre todos los orígenes
- Odoo provee `res.country` y `res.country.state` con datos pre-cargados
- El worker de sync se ejecuta como paso final de `product_sync` (o worker independiente con mismo schedule)
