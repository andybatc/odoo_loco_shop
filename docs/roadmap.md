# Odoo Loco Shop — Roadmap y Mejoras

> Consolidado de los análisis: exploración de código, investigación de mejores prácticas Odoo+Loco.rs, y revisión UX/UI.
> Fecha: 2026-07-02 — Actualizado: 2026-07-02 (sesión fixes críticos + UI)

---

## 1. Resumen Ejecutivo

El proyecto tiene una **base sólida**: catálogo → carrito guest → checkout → Odoo, búsqueda TSVector, auth completo (JWT + magic link), sincronización vía webhooks + worker, caché Redis con invalidación por versión, admin básico, y tests extensos.

Las carencias principales para llegar a producción real son: **pasarela de pago**, **stock real**, **checkout con envío/impuestos**, y **perfil de usuario con historial de órdenes**.

---

## 2. Estado Actual — Inventario de Funcionalidades

### 2.1 Backend (Rust/Loco.rs)

| Feature | Estado | Archivos clave |
|---------|--------|----------------|
| Catálogo de productos | ✅ Completo | `controllers/shop.rs`, `models/products.rs` |
| Paginación server-side | ✅ Completo | `shop/home.html` |
| Filtrado por categoría | ✅ Completo | Query param + Redis cache |
| Búsqueda full-text (TSVector) | ✅ Completo | `shop/search.html`, paginada, español |
| Detalle de producto | ✅ Completo | `product_detail.html` + cache |
| API de búsqueda JSON | ✅ Completa | `/shop/api/search` |
| Carrito guest (cookie-based) | ✅ Completo | `rsv_cart_session`, CRUD |
| Checkout (form + pago metadata) | ✅ Completo | `checkout.rs` + Vue |
| Webhook productos (single/bulk) | ✅ Completo | `products_webhook.rs` |
| Webhook métodos de pago | ✅ Completo | `payment_webhooks.rs` |
| Sincronización full Odoo→Local | ✅ Completo | `workers/product_sync.rs` |
| Tarea CLI sync | ✅ Completa | `tasks/sync.rs` |
| Cleanup carritos abandonados | ✅ Completo | `tasks/cleanup_carts.rs` |
| Auth (JWT, register, login) | ✅ Completo | `controllers/auth.rs` |
| Magic link auth | ✅ Completo | Sin contraseña, email |
| Forgot/reset password | ✅ Completo | Con email |
| Email verification | ✅ Completo | Con token |
| Panel admin (dashboard) | ✅ Completo | `admin/dashboard.html` |
| Lista de órdenes admin | ✅ Completo | `admin/orders.html` con filtros HTMX |
| Config integración (token, URL) | ✅ Completo | `config/ui.html` |
| Caché Redis (catálogo + búsqueda + config) | ✅ Completo | Version bumping, TTL |
| Swagger/OpenAPI | ✅ Completo | `/swagger-ui` |
| Prometheus metrics | ✅ Completo | `/metrics` |
| Rate limiting (parcial) | 🟡 Parcial | Solo auth y webhooks |
| Tests de integración | ✅ Extensos | 15+ archivos |

### 2.2 Odoo Addon (`odoo_rust_sync`)

| Feature | Estado |
|---------|--------|
| Webhook create/write productos | ✅ Completo |
| Webhook payment providers | ✅ Completo |
| Bulk sync desde UI Odoo | ✅ Completo |
| Order API (crea sale.order + invoice) | ✅ Completo |
| Payment methods sync endpoint | ✅ Completo |
| Config settings (token, URL, admin) | ✅ Completo |
| Post-commit webhook dispatch | ✅ Completo |

### 2.3 Frontend (Vue 3 + HTMX + Tailwind v4)

| Feature | Estado |
|---------|--------|
| Home/Landing page | ✅ Completa |
| Catálogo con grid + paginación | ✅ Completo |
| Búsqueda con paginación | ✅ Completa |
| Detalle de producto | ✅ Completo |
| Carrito (Vue reactive) | ✅ Completo |
| Checkout (form + selección pago + resumen) | ✅ Completo |
| Página de orden exitosa | ✅ Completa |
| Login/Register web (HTMX) | ✅ Completo |
| Admin dashboard | ✅ Completo |
| Admin órdenes con filtros | ✅ Completo |
| Config integración | ✅ Completo |
| Menú lateral Vue | ✅ Completo |
| Toast notifications | ✅ Completo |
| Seguridad (CSP, CORS, CSRF) | ✅ Completo |
| Error pages (403/404/500) | ✅ Completo |
| **Mis Órdenes (usuario)** | ❌ Ausente |
| **Wishlist** | ❌ Ausente |
| **Perfil de usuario** | ❌ Ausente |
| **Password reset UI** | ❌ Ausente |

---

## 3. Problemas Críticos — Blockers para Producción Real

### 🔴 Pasarela de pago real
Los métodos de pago se sincronizan desde Odoo como metadata. No hay integración real con Stripe/PayPal/MercadoPago. Sin cobro no se puede operar.

### 🔴 Stock hardcodeado a 0.0 ✅ FIXED
`workers/product_sync.rs` ahora consulta `product_product.qty_available` vía SQL directo. Stock se actualiza correctamente en cada sync.

### 🔴 Checkout sin envío ni impuestos
No hay selección de `delivery.carrier`, no se muestran impuestos (`account.tax`). El total que ve el usuario no es el real.

### 🔴 Sin historial de órdenes para usuarios
La relación `orders.user_id` existe en DB pero no hay vista "Mis Órdenes" ni detalle de orden individual.

### 🟡 Carrito ahora carga para usuarios logueados ✅ FIXED
`cart_display` y `get_cart_items` buscan el carrito por `user_id` (PID) si el usuario está autenticado, en vez de solo por cookie. Pendiente: fusionar carrito guest al hacer login.

---

## 4. Roadmap por Milestones

### Hito 1 — Producción Mínima (Mes 1)

Objetivo: la tienda puede operar y cobrar.

| Tarea | Área | Dependencias |
|-------|------|--------------|
| 1.1 | Integrar pasarela de pago (Stripe/PayPal/MercadoPago) | Backend + Frontend | Odoo payment provider |
| 1.2 | ✅ Sincronizar stock real desde Odoo | Backend | Odoo `stock.quant` / `stock.move` |
| 1.3 | Checkout completo: selección de envío + impuestos visibles | Backend + Frontend | Odoo `delivery.carrier`, `account.tax` |
| 1.4 | Perfil de usuario + historial de órdenes | Frontend + Backend | — |
| 1.5 | 🟡 Carrito funcional para usuarios logueados | Backend | — |
| 1.6 | Rate limiting en todas las rutas públicas | Backend | — |
| 1.7 | Retry queue en Odoo para webhooks | Odoo | — |

### Hito 2 — Crecimiento (Mes 2)

Objetivo: aumentar conversión y retención.

| Tarea | Área | Dependencias |
|-------|------|--------------|
| 2.1 | Cupones y descuentos desde Odoo (pricelists / loyalty) | Backend + Odoo | Odoo `coupon.program` |
| 2.2 | Wishlist / Favoritos | Frontend + Backend | — |
| 2.3 | Variantes de producto (talla/color) | Backend + Frontend | Odoo `product.product` |
| 2.4 | Abandoned cart recovery (email) | Backend + Mailer | Worker programado |
| 2.5 | Búsqueda con autocomplete | Frontend + Backend | — |
| 2.6 | Reconciliation job diario (sync consistencia) | Backend | — |
| 2.7 | ✅ Cachear webhook_token en Redis | Backend | — |
| 2.8 | ✅ Eliminar duplicación de `get_current_user` | Backend | — |
| 2.9 | Cargar Vue solo en páginas que lo necesitan | Frontend | — |

### Hito 3 — Diferenciación (Mes 3+)

Objetivo: features que distinguen la tienda.

| Tarea | Área | Dependencias |
|-------|------|--------------|
| 3.1 | Reseñas y valoraciones de productos | Frontend + Backend | — |
| 3.2 | Blog / CMS básico | Frontend + Backend | — |
| 3.3 | Multi-idioma (español → inglés + otras) | Frontend + Backend | Odoo traducciones |
| 3.4 | Recomendaciones de producto (cross-sell / up-sell) | Backend | Odoo `product.template` |
| 3.5 | SEO completo (sitemap.xml, OG tags, structured data) | Frontend | — |
| 3.6 | Panel de analytics en admin (Chart.js) | Frontend | — |
| 3.7 | PWA básica (service worker, offline catalog) | Frontend | — |
| 3.8 | Newsletter / captura de emails | Frontend + Odoo | Odoo `mailing.contact` |

---

## 5. Mejoras UX/UI Priorizadas

### 🔴 Inmediato (1-2 días)

| # | Problema | Solución |
|---|----------|----------|
| 1 | ✅ Controles de cantidad en carrito (+/-/delete) | Botones vía fetch API + cart.js v1.2 |
| 2 | Sin feedback de carga en navegación | Progress bar tipo YouTube o spinner |
| 3 | ✅ Paleta login/register unificada | login/register → blue-500/slate-900 |
| 4 | Login/register no heredan `base.html` | Heredar layout o crear layout auth compartido |
| 5 | ✅ Favicon inline SVG | `<link rel="icon">` en base.html |
| 6 | ✅ Selector cantidad en product detail | Input numérico + botones +/- |
| 7 | ✅ Error pages sin CDN | 403/404/500 usan tailwind.css local |
| 8 | ✅ Meta description + OG tags | En base.html |

### 🟡 Corto plazo (1 semana)

| # | Problema | Solución |
|---|----------|----------|
| 9 | ✅ Breadcrumbs inline | Tera macro + inline con separator › |
| 10 | Sin skeleton loaders | Cards placeholder animadas (pulse) |
| 11 | Imágenes sin aspect-ratio → layout shift | `aspect-[4/3]` o `aspect-square` |
| 12 | Checkout sin autofill | Datos en cookie/localStorage |
| 13 | Toast minimalista (1 solo, sin tipos) | Stack de toasts con tipos (éxito/error/warning) |
| 14 | Footer genérico | 3-4 columnas (links, contacto, redes) |
| 15 | Banner hero 253KB sin comprimir | Convertir a WebP ~80KB |
| 16 | Sin breadcrumbs en producto/checkout/admin | Agregar partial |

### 🟢 Mediano plazo (2-4 semanas)

| # | Problema | Solución |
|---|----------|----------|
| 17 | Admin sin gestión de productos | CRUD: precio, stock, publicado, sync manual |
| 18 | Search con autocomplete | Vue o HTMX con debounce + dropdown |
| 19 | Modo oscuro | CSS custom properties + localStorage toggle |
| 20 | Responsive audit completo | 320px → 1024px, hamburger menu, grid 1-col mobile |
| 21 | Admin orders sin detalle | Modal o página de detalle de orden |
| 22 | Sin historial de sync Odoo en admin | Tabla con últimos syncs, errores |

---

## 6. Deuda Técnica y Code Smells

### Duplicación de código

| Archivo | Problema |
|---------|----------|
| `controllers/views.rs:25` + `controllers/checkout.rs:46` | ✅ `get_current_user` duplicado eliminado — checkout.rs importa desde views.rs |
| `controllers/config.rs:109` + `controllers/views.rs:241` | URL regex duplicado |
| `controllers/views.rs:207` + `controllers/checkout.rs:76` | Patrón load_cart duplicado (no crítico) |
| `shop/home.html` + `shop/search.html` | Código de paginación casi idéntico |

### Código legacy / muerto

| Archivo | Problema |
|---------|----------|
| `models/product_template_odoo.rs` | Entidad espejo de Odoo (108 líneas, 60+ campos). Solo lectura en sync, nunca se escribe. |
| `models/cart_items.rs` | Stub vacío (`ActiveModelBehavior` trivial) |
| `models/carts.rs` | Stub vacío |
| `models/configs.rs` | Stub casi vacío |
| `models/order_items.rs` | Stub casi vacío |
| `models/orders.rs` | Stub casi vacío |
| `initializers/mod.rs` | Archivo completamente vacío |
| `tailwind.config.js` | Obsoleto en Tailwind v4 (usar `@source` en CSS) |
| `assets/static/index.html` | Placeholder de 0 bytes |

### Seguridad

| Problema | Riesgo |
|----------|--------|
| CSP permite `unsafe-eval` (Vue global build) | Necesario pero abre riesgo XSS |
| CSP permite CDN unpkg.com | No se usa, pero está en la política |
| Rate limiting no cubre `/api/carts` ni `/api/checkout` | Abuso / brute force |
| `product_sync.rs` usa conexión DB directa a Odoo | Acoplamiento fuerte, riesgo de exposición |
| ✅ `AuthToken` middleware ahora usa `config_cache::get_cached_config` (Redis) | Ya no consulta DB por request |
| `check_rate_limit` en webhooks crea conexión Redis directa | Debería usar `ctx.cache` |

### Frontend

| Problema | Impacto |
|----------|---------|
| `vue.global.prod.js` (165KB) se carga en todas las páginas | Peso innecesario donde no se usa Vue |
| `htmx.min.js` se carga en todas las páginas | Podría cargarse condicionalmente |
| No hay service worker / PWA | App no instalable |
| No hay skeleton loaders | Experiencia de carga abrupta |

---

## 7. Arquitectura de Integración Odoo ↔ Rust (Estado vs Recomendado)

### Estado actual
```
Odoo → Webhook HTTP (post-commit, fire-and-forget) → Rust API → Worker Redis → DB Shop
Rust → HTTP POST /api/orders/create (síncrono) → Odoo API
```

### Problemas identificados
- Si Rust está caído, el webhook se pierde silenciosamente (sin retry)
- Webhooks son síncronos dentro del POST de Odoo → bloquea al usuario si Rust tarda
- No hay reconciliation job → datos inconsistentes si un webhook se pierde
- Sync masivo lee DB directa de Odoo → acoplamiento fuerte, puede romperse en upgrades
- No hay idempotencia en webhooks → race conditions si llegan duplicados

### Arquitectura recomendada
```
Odoo:
  Product/Stock/Payment → rust_sync.queue (DB table con retries)
                          → postcommit.add() / cron cada 1 min
                          → HTTP POST + Bearer token → Rust

Rust:
  POST /api/webhooks → WebhookWorker (Redis) → Update DB + invalidar caché
  Reconciliation Worker (cron diario) → Odoo External API (no DB directa)
```

---

## 8. Configuración de Producción (Loco.rs)

Pendiente crear `config/production.yaml`:

```yaml
logger:
  enable: true
  pretty_backtrace: false
  level: info
  format: json

database:
  max_connections: 50
  enable_logging: false
  auto_migrate: true

workers:
  mode: BackgroundQueue

queue:
  kind: Redis
  uri: ${REDIS_URL}
  dangerously_flush: false

cache:
  kind: Redis
  uri: ${REDIS_URL}
```

Workers en producción: servidor web + workers separados:

```
./target/release/odoo_shop-cli start          # servidor
./target/release/odoo_shop-cli start --worker # workers (otra instancia)
```

Workers adicionales necesarios:
- `AbandonedCartWorker` — email recovery tras N horas
- `OrderStatusWorker` — polling de estado de pedidos a Odoo
- `ReconciliationWorker` — sync periódico de todo el catálogo
- `InventorySyncWorker` — actualización periódica de stock
