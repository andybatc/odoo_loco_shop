# Agents.md - Odoo Loco Shop

## Stack

- **Backend**: Rust (edition 2021), Loco.rs 0.16 (Axum 0.8), SeaORM 1.1, PostgreSQL, Redis
- **Frontend**: Vue 3 (global build), HTMX, Tailwind CSS, Tera templates
- **ERP**: Odoo 18.0 (Python 3.12)
- **Auth**: JWT (Loco), magic-link, cookie-based guest carts
- **CI**: GitHub Actions (rustfmt + clippy + tests)

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

### Testing

- Tests use `request::<App, _, _>(|request, _ctx| async move { ... })` pattern
- Tests annotated with `#[tokio::test] #[serial]`
- Run: `cargo test --all-features --all`

### CRITICAL rules

- NEVER add comments to code unless the user explicitly asks
- NEVER create documentation (*.md) or README files unless explicitly requested
- Always use `i32` for Odoo product IDs, `Uuid` for internal IDs
- Cookie-based guest cart key: `rsv_cart_session`
- Webhook auth: Bearer token stored in `configs` table as `webhook_token`
- Use `tracing::info!()` for logging, not `println!`
- Prefer `edit` tool over `write` for existing files
- NEVER commit unless user explicitly asks
- Always run `cargo test --all-features --all` after making changes (if feasible)
