# Odoo Loco Shop

[![CI](https://github.com/andybatc/odoo_loco_shop/actions/workflows/ci.yaml/badge.svg)](https://github.com/andybatc/odoo_loco_shop/actions/workflows/ci.yaml)
[![Odoo](https://img.shields.io/badge/Odoo-18.0-875A7B.svg?logo=odoo&logoColor=white)](https://www.odoo.com)
[![Loco](https://img.shields.io/badge/Loco-0.16-CRIMSON.svg?logo=rust&logoColor=white)](https://loco.rs)
[![Rust](https://img.shields.io/badge/Rust-2021-dea584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Redis](https://img.shields.io/badge/Redis-DC382D.svg?logo=redis&logoColor=white)](https://redis.io)
[![License: LGPL-3](https://img.shields.io/badge/License-LGPL--3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.html)

Catálogo web de alto rendimiento con Odoo 18 como ERP y un frontend/backend
desacoplado en Rust (Loco.rs) con Redis para caché y colas.

---

## Tabla de Contenidos

- [Stack](#stack)
- [Arquitectura](#arquitectura)
- [Caracteristicas](#caracteristicas)
- [Estructura del Proyecto](#estructura-del-proyecto)
- [Requisitos](#requisitos)
- [Instalacion](#instalacion)
  - [Backend Rust](#backend-rust)
  - [Addons Odoo](#addons-odoo)
- [Configuracion](#configuracion)
- [Uso](#uso)
- [API](#api)
- [Testing](#testing)
- [Licencia](#licencia)

---

## Stack

| Componente | Tecnologia |
|------------|-----------|
| Backend web | Rust 2021, Loco.rs 0.16 (Axum 0.8), SeaORM 1.1 |
| Frontend | Vue 3 (global build), Tailwind CSS, Tera templates |
| ERP | Odoo 18.0 (Python 3.12) |
| Base de datos | PostgreSQL 13+ |
| Cache / Colas | Redis 6+ |
| CI | GitHub Actions (rustfmt + clippy + tests) |

---

## Arquitectura

```
Navegador ──► Rust Backend (:5150) ──► PostgreSQL (shop)
                    │
                    │  (HTTP webhook + sync directo DB)
                    ▼
               Odoo 18 (:8069) ──► PostgreSQL (odoo)
```

- El sitio web corre en Rust (Loco.rs), no en Odoo.
- Los productos se sincronizan desde Odoo al backend local via webhooks
  (Odoo -> Rust) o mediante sincronizacion masiva via DB directa (CLI).
- Redis se usa para cachear el catalogo de productos y como cola de
  workers asincronos.
- Los pedidos se crean en Odoo desde Rust via HTTP (endpoint propio con
  autenticacion por Bearer token).

---

## Caracteristicas

- **Catalogo web**: Listado de productos con busqueda, cacheado en Redis.
- **Sincronizacion Odoo**: Webhooks en creacion/actualizacion de productos
  desde Odoo al backend Rust.
- **Sincronizacion masiva**: CLI task que lee la base de datos de Odoo
  directamente y pobla la tabla local.
- **Carrito de compras**: Carrito persistente via cookie (`rsv_cart_session`)
  con soporte para usuarios autenticados e invitados.
- **Checkout**: Pagina de confirmacion con formulario de datos de envio.
  Al confirmar, se crea una orden de venta en Odoo con factura validada.
- **Autenticacion**: JWT (Loco), magic-link, registro y login via formularios
  web o API JSON.
- **Gestor de configuracion**: UI web para configurar el token de
  autenticacion de los webhooks.
- **Tema Odoo**: Conjunto de addons MuK para personalizar la interfaz
  backend de Odoo (sidebar, colores, dialogs, chatter).

---

## Estructura del Proyecto

```
rust_backend/               # Backend Rust (Loco.rs)
  src/
    bin/main.rs             # Entry point
    app.rs                  # Hooks: rutas, workers, tareas
    lib.rs                  # Modulos
    controllers/
      auth.rs               # API auth (login, register, magic-link)
      carts.rs              # API carrito (POST /api/carts)
      checkout.rs           # Checkout + confirmacion y success page
      config.rs             # API configuracion (webhook token)
      homepage.rs           # Pagina de inicio
      products_webhook.rs   # Webhooks Odoo -> Rust
      shop.rs               # Catalogo de productos
      token_auth.rs         # Extractor de token Bearer
      views.rs              # Paginas web (login, register, carrito, config)
    models/                 # Modelos SeaORM + _entities/ (codegen)
    workers/
      webhook.rs            # Worker que procesa webhooks de Odoo
      product_sync.rs       # Worker de sincronizacion masiva
    tasks/
      sync.rs               # CLI task para sincronizar productos
  config/
    development.yaml        # Config de desarrollo
    test.yaml               # Config de testing
  migration/                # Migraciones SeaORM (8 tablas)
  assets/
    views/                  # Plantillas Tera
    static/                 # CSS, JS, imagenes
  storage/products/         # Imagenes de productos
  tests/                    # Tests de integracion

odoo_custom_addons/
  odoo_rust_sync/           # Addon de sincronizacion Odoo -> Rust
  muk_web_theme/            # Tema backend Odoo
  muk_web_dialog/           # Dialogos a pantalla completa
  muk_web_chatter/          # Mejoras en el chatter
  muk_web_colors/           # Personalizacion de colores
  muk_web_appsbar/          # Barra lateral de apps
```

---

## Requisitos

- Rust 2021 edition + Cargo
- PostgreSQL 13+
- Redis 6+
- Python 3.12
- Odoo 18.0 (Community o Enterprise) con los modulos `sale` y `account`
  instalados

---

## Instalacion

### Backend Rust

```bash
# Clonar el repositorio
git clone git@github.com:andybatc/odoo_loco_shop.git
cd odoo_loco_shop/rust_backend

# Configurar variables de entorno (o usar defaults)
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/odoo_shop_development"
export REDIS_URL="redis://127.0.0.1"
export ODOO_DATABASE_URL="postgres://odoo:postgres@localhost:5432/odoo_prod"

# Iniciar servidor de desarrollo
cargo loco start
```

El servidor arranca en `http://localhost:5150` con migraciones automaticas
(`auto_migrate: true`).

### Addons Odoo

```bash
# En el directorio de addons personalizados de Odoo
cd /opt/odoo/custom_addons

# Clonar (o enlazar) el repositorio
ln -s /ruta/a/odoo_loco_shop/odoo_custom_addons/odoo_rust_sync .
ln -s /ruta/a/odoo_loco_shop/odoo_custom_addons/muk_web_* .

# Agregar la ruta al --addons-path de Odoo
# Ejemplo:
./odoo-bin --addons-path=/opt/odoo/custom_addons,/opt/odoo/addons \
           -d odoo_prod \
           -i odoo_rust_sync,muk_web_theme,muk_web_dialog,muk_web_chatter,muk_web_colors,muk_web_appsbar
```

---

## Configuracion

### Token de Webhook

El addon `odoo_rust_sync` genera automaticamente un token
(`rust_api.webhook_token`) en los parametros de sistema de Odoo.
Se puede visualizar y modificar desde el backend Rust en:

```
http://localhost:5150/ui/auth/token
```

O via API:

```bash
# Obtener token actual
curl http://localhost:5150/api/config/token

# Actualizar token
curl -X POST http://localhost:5150/api/config/token \
  -H "Content-Type: application/json" \
  -d '{"token": "nuevo_token"}'
```

### Variables de entorno

| Variable | Default | Descripcion |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/odoo_shop_development` | Base de datos local |
| `REDIS_URL` | `redis://127.0.0.1` | Servidor Redis |
| `ODOO_DATABASE_URL` | `postgres://odoo:postgres@localhost:5432/odoo_prod` | Base de datos de Odoo (sync masivo) |

---

## Uso

### Sincronizacion de productos

Los productos se sincronizan automaticamente desde Odoo al backend Rust
cuando se crean o modifican en Odoo (webhook).

Para sincronizacion masiva inicial:

```bash
cd rust_backend
cargo loco task sync
```

### Catalogo web

- **Inicio**: `http://localhost:5150/`
- **Productos**: `http://localhost:5150/shop/home`
- **Carrito**: `http://localhost:5150/cart`
- **Checkout**: `http://localhost:5150/checkout`

### Flujo de compra

1. El usuario navega el catalogo y agrega productos al carrito.
2. En `/cart` revisa los items y hace clic en "Proceder al Pago".
3. En `/checkout` completa sus datos de contacto y confirma el pedido.
4. El backend Rust envia los datos a Odoo via HTTP.
5. Odoo crea el partner, la orden de venta, la confirma, genera la
   factura y la valida.
6. El usuario es redirigido a `/order/success` con la referencia de
   la orden y la factura.
7. El carrito se limpia (DB + cookie).

---

## API

### Webhooks (Odoo -> Rust)

| Metodo | Ruta | Descripcion |
|--------|------|-------------|
| POST | `/api/webhooks/odoo/update` | Producto individual |
| POST | `/api/webhooks/odoo/bulk-update` | Productos en lote |

Requieren header `Authorization: Bearer <token>` (token de `configs.webhook_token`).

### Carrito

| Metodo | Ruta | Descripcion |
|--------|------|-------------|
| POST | `/api/carts/` | Agregar item (body: `{"product_id": 123}`) |

### Checkout

| Metodo | Ruta | Descripcion |
|--------|------|-------------|
| POST | `/api/checkout` | Procesar pedido (body: `{"customer": {...}}`) |

### Autenticacion

| Metodo | Ruta | Descripcion |
|--------|------|-------------|
| POST | `/api/auth/register` | Registro de usuario |
| POST | `/api/auth/login` | Inicio de sesion |
| POST | `/api/auth/magic-link` | Magic link |
| GET | `/api/auth/current` | Usuario actual |

### Configuracion

| Metodo | Ruta | Descripcion |
|--------|------|-------------|
| GET | `/api/config/token` | Obtener webhook token |
| POST | `/api/config/token` | Actualizar webhook token |

---

## Testing

```bash
cd rust_backend

# Ejecutar todos los tests
cargo test --all-features --all

# Linter
cargo clippy --all-features

# Formato
cargo fmt --all -- --check
```

Los tests requieren PostgreSQL y Redis corriendo.

---

## Licencia

LGPL-3.0. Ver [LICENSE](./LICENSE).
