# Agents.md - Odoo Loco Shop

## Stack

- **Backend**: Rust (edition 2021), Loco.rs 0.16 (Axum 0.8), SeaORM 1.1, PostgreSQL, Redis
- **Frontend**: Vue 3 (global build, prod), HTMX 1.9, Tailwind CSS v4, Tera templates
- **ERP**: Odoo 18.0 (Python 3.12)
- **Auth**: JWT (Loco), magic-link, cookie-based guest carts
- **CI**: GitHub Actions (rustfmt + clippy + tests)

## Odoo

- **Path**: `~/Odoo/odoo-18.0/odoo-18.0/`
- **Virtual env**: `~/Entornos virtuales/odoo18/.venv/` (UV-managed)
- **Activation** (fish): `source "Entornos virtuales/odoo18/.venv/bin/activate.fish"`
- **Activation** (bash): `source "Entornos virtuales/odoo18/.venv/bin/activate"`
- **Run**: `./odoo-bin -c odoo_prod.conf` (desde la raíz de Odoo, con venv activado)
- **prometheus-client** ya instalado en el venv

## Running

### Start backend (with queue worker)

Loco `start` por defecto corre en **ServerOnly** — los `BackgroundWorker` que usan Redis queue NO se ejecutan.

```bash
cd rust_backend && setsid -f bash -c 'exec target/debug/odoo_shop-cli start --server-and-worker' &>/tmp/rust_shop.log
```

La flag `--server-and-worker` spawnée 2 worker threads que procesan `queue:default` en Redis. Sin esta flag, `perform_later()` encola jobs en Redis pero nadie los procesa.

### Start Odoo

```bash
cd ~/Odoo/odoo-18.0/odoo-18.0 && source "/home/andy/Entornos virtuales/odoo18/.venv/bin/activate" && ./odoo-bin -c odoo_prod.conf
```

## Project structure

```
rust_backend/             # Rust/Loco backend
  src/
    bin/main.rs           # Entry point
    app.rs                # Loco Hooks (routes, workers, tasks)
    lib.rs                # Module declarations
    controllers/          # Axum route handlers
    models/               # SeaORM models + _entities/ (codegen)
    workers/              # Background workers (Redis queue)
    tasks/                # CLI tasks (sync.rs)
    mailers/              # AuthMailer
    views/                # Response view structs
  config/                 # development.yaml, test.yaml
  migration/              # SeaORM migrations
  tests/                  # Integration + unit tests
  assets/                 # Static assets (CSS, JS, images, Tera templates)
  storage/products/       # Product images on disk
odoo_custom_addons/       # Odoo 18.0 addons (muk_web_*, odoo_rust_sync)
```

## Conventions

### Rust code style

- `max_width = 100`, `use_small_heuristics = "Default"` (`.rustfmt.toml`)
- Use `#![allow(clippy::missing_errors_doc)]` at the top of controller files
- Use `use loco_rs::prelude::*` for common imports
- Use `use sea_orm::ActiveValue::Set` for insert/update operations
- Use `#[debug_handler]` on controller functions
- Use `#[async_trait::async_trait]` on model `ActiveModelBehavior` impls

### Project patterns

- Routes defined with `pub fn routes() -> Routes` in each controller
- Controllers use `State(ctx): State<AppContext>` pattern
- Models: `_entities/` contains SeaORM codegen output; hand-written logic in parent module files
- Caching: use `ctx.cache` (Redis) for product catalog; key pattern `"products:all"`
- Config stored in `configs` DB table accessed via key/value pattern

### Frontend conventions (HTMX/Vue separation)

- **HTMX**: server-driven (POST/DELETE que cambian estado: add-to-cart, auth, logout)
- **Vue 3**: UI reactiva client-side (menú lateral, carrito, checkout, búsqueda)
- Comunicación HTMX ↔ Vue vía CustomEvents: `update-cart-count`, `abrir-menu-rust`
- shop.js está eliminado (dead code, su addToCart duplicaba HTMX)
- Todo `add-to-cart` usa HTMX con `hx-post="/api/carts"` + `hx-vals`
- register.html usa `hx-ext="json-enc"` + `/static/js/json-enc.js` porque el handler Loco espera JSON
- Data del server → `<script type="application/json">` (no data-attributes, evita escape bugs)

### Tailwind CSS v4

- Build command: `npx @tailwindcss/cli -i assets/static/css/tailwind-input.css -o assets/static/css/tailwind.css --minify`
- Input CSS usa `@import "tailwindcss"` + `@source` (no `@tailwind base/components/utilities`)
- Config en `tailwind.config.js` con `content:` paths (opcional en v4, auto-scan con `@source`)
- Templates usan `<link href="/static/css/tailwind.css" rel="stylesheet">` (no script CDN)
- register.html y login.html son standalone (no heredan base.html) — cada uno necesita sus propios `<link>` y `<script>`
- `npm install -D tailwindcss @tailwindcss/cli` para instalar

### Vue 3 global build

- Usar `vue.global.prod.js` (165KB, sin warnings de dev)
- Requiere `'unsafe-eval'` en CSP (template compiler vía new Function())
- No hay Runtime-only build — el global build compila templates en runtime

### Frontend critical rules

- Todos los scripts JS/assets locales (NO CDN: ni Tailwind, ni HTMX, ni Vue)
- shop.js está eliminado y NO debe reintroducirse
- No agregar addToCart vía Vue/fetch en home.html — siempre HTMX
- Tras cambios en templates, rebuild Tailwind: `npx @tailwindcss/cli -i ... -o ... --minify`

### Testing

- NEVER add comments to code unless the user explicitly asks
- NEVER create documentation (*.md) or README files unless explicitly requested
- Always use `i32` for Odoo product IDs, `Uuid` for internal IDs
- Cookie-based guest cart key: `rsv_cart_session`
- Webhook auth: Bearer token stored in `configs` table as `webhook_token`
- Use `tracing::info!()` for logging, not `println!`
- Prefer `edit` tool over `write` for existing files
- NEVER commit unless user explicitly asks
- Always run `cargo test --all-features --all` after making changes (if feasible)
