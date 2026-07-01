# Development

## Prerequisites

```bash
# Backend
rustup default stable
cargo install loco-cli

# Herramientas
cargo install cargo-watch
```

## Commands

```bash
# Iniciar servidor con hot-reload
cd rust_backend
cargo loco start

# Tests
cargo test --all-features --all

# Linter
cargo clippy --all-features

# Formato
cargo fmt --all -- --check
```

## Project Structure

```
rust_backend/
  src/
    bin/main.rs           # Entry point
    app.rs                # Hooks: rutas, workers, tareas
    lib.rs                # Módulos
    controllers/          # Axum route handlers
      auth.rs             # API auth (login, register, magic-link)
      carts.rs            # API carrito
      checkout.rs         # Checkout + confirmación y success page
      config.rs           # API configuración (webhook token)
      homepage.rs         # Página de inicio
      products_webhook.rs # Webhooks Odoo -> Rust
      shop.rs             # Catálogo de productos
      views.rs            # Páginas web (login, register, carrito, config)
      admin.rs            # Panel admin
      payment_webhooks.rs # Webhooks de métodos de pago
    models/               # Modelos SeaORM + _entities/ (codegen)
    workers/
      webhook.rs          # Worker que procesa webhooks de Odoo
      product_sync.rs     # Worker de sincronización masiva
    tasks/
      sync.rs             # CLI task para sincronizar productos
      promote_user.rs     # CLI task para promocionar usuario a admin
      cleanup_carts.rs    # CLI task para limpiar carritos expirados
    mailers/
      auth.rs             # AuthMailer (magic-link)
      order.rs            # OrderMailer (confirmación de pedido)
    middleware/
      security_headers.rs # Security headers middleware
      auth_extractor.rs   # Bearer token extractor
      csrf.rs             # CSRF protection
    views/
      auth.rs             # View helpers para auth forms
  config/
    development.yaml      # Config de desarrollo
    test.yaml             # Config de testing
  migration/              # SeaORM migrations
  assets/
    views/                # Plantillas Tera
    static/               # CSS, JS, imágenes
  storage/products/       # Imágenes de productos
  tests/                  # Tests de integración

odoo_custom_addons/
  odoo_rust_sync/         # Addon de sincronización Odoo -> Rust
  muk_web_theme/          # Tema backend Odoo
  muk_web_dialog/         # Diálogos a pantalla completa
  muk_web_chatter/        # Mejoras en el chatter
  muk_web_colors/         # Personalización de colores
  muk_web_appsbar/        # Barra lateral de apps
```

## Testing

```bash
# Todos los tests
cargo test --all-features --all

# Tests específicos
cargo test checkout -- --nocapture

# Con logging
RUST_LOG=debug cargo test
```

Los tests requieren PostgreSQL y Redis corriendo. Usan la DB configurada en `config/test.yaml`.

## Adding a Route

1. Crear controlador en `src/controllers/` con `pub fn routes() -> Routes`
2. Registrar en `src/app.rs` → `App::routes()`
3. Agregar modelo en `src/models/` si aplica
4. Agregar migración en `migration/` si aplica

## Adding a Worker

1. Crear worker en `src/workers/` implementando `BackgroundWorker`
2. Registrar en `src/app.rs` → `App::connect_workers()`

## Style Guide

- `max_width = 100`, `use_small_heuristics = "Default"` (ver `.rustfmt.toml`)
- `#![allow(clippy::missing_errors_doc)]` en controllers
- `use loco_rs::prelude::*` para imports comunes
- `use sea_orm::ActiveValue::Set` para insert/update
- `#[debug_handler]` en funciones de controller
- `i32` para IDs de Odoo, `Uuid` para IDs internos
- Cookie de carrito invitado: `rsv_cart_session`
- Usar `tracing::info!()` para logging, no `println!`
