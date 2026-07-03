# Order Creation Background Worker

> Hace asíncrona la creación de órdenes en Odoo vía Redis queue.
> Fecha: 2026-07-02

---

## Resumen

Actualmente `submit_checkout` hace HTTP POST síncrono a Odoo para crear
`sale.order` + invoice. El usuario espera la respuesta. Si Odoo está lento o
cae, la compra falla.

Se mueve la llamada Odoo a un `BackgroundWorker` de Loco (cola Redis). El
checkout responde instantáneo, el worker procesa en background con reintentos.

---

## Flujo Nuevo

```
Browser → POST /api/checkout
  → Crea orden + items en DB (status="pending")
  → OrderCreationWorker::perform_later(&ctx, args)  ← Redis queue
  → Responde "success" al toque

Worker (background):
  → Carga orden + items de DB
  → HTTP POST a Odoo /api/orders/create  (hasta 3 intentos)
  → Éxito: status="confirmed", guarda odoo_order_name, odoo_invoice_name
  → Fracaso: status="failed", log del error
```

---

## Archivos a modificar/crear

### Nuevo: `src/workers/order_creation.rs`

Worker con:
- **Payload** (`OrderWorkerArgs`): `order_id: Uuid`
- **`perform()`**:
  1. Carga `orders::Model` + `order_items` de DB
  2. Lee `webhook_token` + `odoo_base_url` de configs (misma lógica que hoy en checkout.rs L253-268)
  3. Construye payload JSON: `{ customer: {name, email, phone, street, city, zip}, items: [{product_tmpl_id, name, price, quantity}], payment_method_id }`
  4. HTTP POST con `reqwest::Client`, timeout 15s, 3 intentos:
     - Intento 1: inmediato
     - Intento 2: sleep 30s
     - Intento 3: sleep 5min
  5. Éxito (200): parsea `{order_name, order_id, invoice_name, invoice_id}`, actualiza `orders::ActiveModel { status: Set("confirmed"), odoo_order_name, odoo_invoice_name }`
  6. Error (HTTP no-200, timeout, conexión): log, espera, reintenta
  7. 3 fallos: `status = Set("failed")`
  8. Antes de cualquier HTTP, verifica que `order.status == "pending"` — si ya no lo está (confirmado por otro worker), skip

### Modificado: `src/controllers/checkout.rs`

`submit_checkout` se reduce:

```
1. Valida email/name (se queda igual)
2. Lee carrito + items (se queda igual)
3. Crea orden + items en DB (se queda igual)
4. Encola OrderCreationWorker  ← NUEVO, reemplaza bloque HTTP Odoo
5. Limpia carrito + cookie     ← se mueve aquí (antes era post-Odoo)
6. Responde success inmediato  ← ya no espera
```

Se elimina:
- Toda la lógica HTTP a Odoo (L253-292 aprox)
- Manejo de respuesta exitosa (L342-357)
- Manejo de error Odoo (L294-339, L383-400)
- `reqwest` import si queda sin usarse

### Modificado: `src/app.rs`

Registrar el worker:

```rust
queue.register(crate::workers::order_creation::OrderCreationWorker::build(ctx)).await?;
```

### No cambia

- Modelos DB (`orders`, `order_items`, `products`)
- Odoo Python endpoint (`controllers.py`)
- Frontend (solo el mensaje de éxito puede ajustarse)
- Config (`development.yaml`)

---

## Retry Logic

```rust
let max_retries = 3;
let delays = [Duration::from_secs(0), Duration::from_secs(30), Duration::from_secs(300)];

for attempt in 0..max_retries {
    if attempt > 0 {
        tokio::time::sleep(delays[attempt]).await;
    }
    match call_odoo(&self.ctx, &order, &items).await {
        Ok(response) => { /* success path */ return Ok(()); }
        Err(e) => tracing::warn!("Intento {} falló: {:?}", attempt + 1, e),
    }
}
// 3 fallos → status = "failed"
```

---

## Edge Cases

| Caso | Comportamiento |
|------|---------------|
| Odoo caído 3 intentos | Orden queda `failed`. Admin puede re-intentar manual |
| Orden ya no existe al ejecutar worker | Log error, no crash |
| Payload inválido (falta product_tmpl_id) | Odoo responde 400, log, orden `failed` |
| Worker crashea durante HTTP | Loco BackgroundQueue no re-entrega (at-most-once). Orden queda `pending` |
| Dos workers para misma orden | No debería pasar (solo se encola una vez), pero `find_by_id` + `status = pending` previene duplicados |

---

## No incluido (scope explícito)

- Email de confirmación de orden
- Callback Odoo→Loco
- Re-intento manual desde admin UI
- Re-encolado automático (worker agota 3 intentos y falla)
- Dashboard de workers fallidos
