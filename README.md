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

## Quick Start

```bash
# Opción 1: Docker (recomendado)
cp .env.example .env
docker compose up
# Servidor en http://localhost:5150

# Opción 2: Manual (requiere Rust + PostgreSQL + Redis)
cd rust_backend
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/odoo_shop_development"
export REDIS_URL="redis://127.0.0.1"
cargo loco start
```

Servidor en `http://localhost:5150`. Documentación interactiva de la API en [`http://localhost:5150/swagger-ui`](http://localhost:5150/swagger-ui).

Para levantar también Odoo (con módulos pre-instalados):
```bash
docker compose --profile full up
```
Odoo disponible en `http://localhost:8069`.

---

## Tabla de Contenidos

- [Quick Start](#quick-start)
- [Stack](#stack)
- [Arquitectura](#arquitectura)
- [Características](#caracteristicas)
- [Estructura del Proyecto](#estructura-del-proyecto)
- [Requisitos](#requisitos)
- [Instalación](#instalacion)
  - [Backend Rust](#backend-rust)
  - [Addons Odoo](#addons-odoo)
- [Configuración](#configuración)
- [Uso](#uso)
- [API](#api)
- [Testing](#testing)
- [Contribuir](#contribuir)
- [Licencia](#licencia)

---

## Stack

| Componente | Tecnología |
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
  (Odoo -> Rust) o mediante sincronización masiva via DB directa (CLI).
- Redis se usa para cachear el catálogo de productos y como cola de
  workers asíncronos.
- Los pedidos se crean en Odoo desde Rust via HTTP (endpoint propio con
  autenticación por Bearer token).

---

## Características

- **Catálogo web**: Listado de productos con búsqueda, cacheado en Redis.
- **Sincronización Odoo**: Webhooks en creación/actualización de productos
  desde Odoo al backend Rust.
- **Sincronización masiva**: CLI task que lee la base de datos de Odoo
  directamente y pobla la tabla local.
- **Carrito de compras**: Carrito persistente via cookie (`rsv_cart_session`)
  con soporte para usuarios autenticados e invitados.
- **Checkout**: Página de confirmación con formulario de datos de envío.
  Al confirmar, se crea una orden de venta en Odoo con factura validada.
- **Autenticación**: JWT (Loco), magic-link, registro y login via formularios
  web o API JSON.
- **Gestor de configuración**: UI web para configurar el token de
  autenticación de los webhooks.
- **Tema Odoo**: Conjunto de addons MuK para personalizar la interfaz
  backend de Odoo (sidebar, colores, dialogs, chatter).

---

## Estructura del Proyecto

```
rust_backend/               # Backend Rust (Loco.rs)
  src/
    bin/main.rs             # Entry point
    app.rs                  # Hooks: rutas, workers, tareas
    lib.rs                  # Módulos
    controllers/
      auth.rs               # API auth (login, register, magic-link)
      carts.rs              # API carrito (POST /api/carts)
      checkout.rs           # Checkout + confirmación y success page
      config.rs             # API configuración (webhook token)
      homepage.rs           # Página de inicio
      products_webhook.rs   # Webhooks Odoo -> Rust
      shop.rs               # Catálogo de productos
      token_auth.rs         # Extractor de token Bearer
      views.rs              # Páginas web (login, register, carrito, config)
    models/                 # Modelos SeaORM + _entities/ (codegen)
    workers/
      webhook.rs            # Worker que procesa webhooks de Odoo
      product_sync.rs       # Worker de sincronización masiva
    tasks/
      sync.rs               # CLI task para sincronizar productos
  config/
    development.yaml        # Config de desarrollo
    test.yaml               # Config de testing
  migration/                # Migraciones SeaORM (8 tablas)
  assets/
    views/                  # Plantillas Tera
    static/                 # CSS, JS, imágenes
  storage/products/         # Imágenes de productos
  tests/                    # Tests de integración

odoo_custom_addons/
  odoo_rust_sync/           # Addon de sincronización Odoo -> Rust
  muk_web_theme/            # Tema backend Odoo
  muk_web_dialog/           # Diálogos a pantalla completa
  muk_web_chatter/          # Mejoras en el chatter
  muk_web_colors/           # Personalización de colores
  muk_web_appsbar/          # Barra lateral de apps
```

---

## Requisitos

- Rust 2021 edition + Cargo
- PostgreSQL 13+
- Redis 6+
- Python 3.12
- Odoo 18.0 (Community o Enterprise) con los módulos `sale` y `account`
  instalados

---

## Instalación

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

El servidor arranca en `http://localhost:5150` con migraciones automáticas
(`auto_migrate: true`).

### Odoo + Docker (recomendado)

Al usar `docker compose --profile full up`, el servicio Odoo se configura automáticamente:

1. **Build**: `odoo_custom_addons/Dockerfile` extiende la imagen oficial `odoo:18` e instala la librería Python `requests` (necesaria para los webhooks salientes de `odoo_rust_sync`).
2. **Montaje de addons**: el directorio `./odoo_custom_addons/` se monta como volumen en `/mnt/extra-addons/` dentro del contenedor. La imagen Odoo incluye ese path en su `--addons-path` por defecto.
3. **Auto-inicialización**: en el primer arranque, el comando `-d odoo_prod -i muk_web_theme,odoo_rust_sync` crea la base de datos e instala todos los módulos automáticamente.
4. **Red interna**: Odoo alcanza al backend Rust via `http://rust_backend:5150` (nombre del servicio Docker).

```yaml
# Extracto de docker-compose.yml
volumes:
  - ./odoo_custom_addons:/mnt/extra-addons  # addons montados aquí
command: odoo -d odoo_prod -i odoo_rust_sync,muk_web_theme --without-demo=all
```

### Addons Odoo (instalación manual, sin Docker)

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

> **Nota**: en la instalación manual también necesitás instalar `requests` en el entorno Python de Odoo: `pip install requests`.

---

## Configuración

### Variables de entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/odoo_shop_development` | Base de datos local |
| `REDIS_URL` | `redis://127.0.0.1` | Servidor Redis |
| `ODOO_DATABASE_URL` | `postgres://odoo:postgres@localhost:5432/odoo_prod` | Base de datos de Odoo (sync masivo) |

Ver [`.env.example`](.env.example).

### Webhook Odoo → Rust

Para que Odoo se comunique con el backend Rust necesita dos parámetros:

| Parámetro | Valor ejemplo | Dónde se configura |
|-----------|---------------|-------------------|
| `rust_api.base_url` | `http://rust_backend:5150` | Odoo → Ajustes → Técnico → Parámetros del sistema |
| `rust_api.webhook_token` | (generado automáticamente) | Odoo → Ajustes → Técnico → Parámetros del sistema, o en `http://localhost:5150/ui/auth/token` |

Pasos:

1. **Con Docker**: el `base_url` es `http://rust_backend:5150` (nombre del servicio en la red interna).
   **Sin Docker**: usar `http://host.docker.internal:5150` (o `localhost:5150` si Odoo corre nativo).
2. El token se genera automáticamente al abrir Ajustes → Módulos Rust Sync en Odoo.
   Alternativamente, se puede crear manualmente o sincronizar desde la UI del backend Rust en `http://localhost:5150/ui/auth/token`.

```bash
# Ver token actual
curl http://localhost:5150/api/config/token

# Actualizar token
curl -X POST http://localhost:5150/api/config/token \
  -H "Content-Type: application/json" \
  -d '{"token": "nuevo_token"}'
```

### Verificar que la integración funciona

1. En Odoo, crear o editar un producto →  guardar.
2. El addon `odoo_rust_sync` dispara un webhook HTTP POST a `{base_url}/api/webhooks/odoo/update` con los datos del producto.
3. Ver en logs de Rust: `tracing::info!()` muestra "Webhook received".
4. Confirmar navegando a `http://localhost:5150/shop/home` — el producto aparece.

---

## Uso

### Sincronización de productos

Los productos se sincronizan automáticamente desde Odoo al backend Rust
cuando se crean o modifican en Odoo (webhook).

Para sincronización masiva inicial:

```bash
cd rust_backend
cargo loco task sync
```

### Catálogo web

- **Inicio**: `http://localhost:5150/`
- **Productos**: `http://localhost:5150/shop/home`
- **Carrito**: `http://localhost:5150/cart`
- **Checkout**: `http://localhost:5150/checkout`

### Flujo de compra

1. El usuario navega el catálogo y agrega productos al carrito.
2. En `/cart` revisa los items y hace clic en "Proceder al Pago".
3. En `/checkout` completa sus datos de contacto y confirma el pedido.
4. El backend Rust envía los datos a Odoo via HTTP.
5. Odoo crea el partner, la orden de venta, la confirma, genera la
   factura y la valida.
6. El usuario es redirigido a `/order/success` con la referencia de
   la orden y la factura.
7. El carrito se limpia (DB + cookie).

---

## API

### Webhooks (Odoo -> Rust)

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/webhooks/odoo/update` | Producto individual |
| POST | `/api/webhooks/odoo/bulk-update` | Productos en lote |

Requieren header `Authorization: Bearer <token>` (token de `configs.webhook_token`).

### Carrito

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/carts/` | Agregar item (body: `{"product_id": 123}`) |

### Checkout

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/checkout` | Procesar pedido (body: `{"customer": {...}}`) |

### Autenticación

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/auth/register` | Registro de usuario |
| POST | `/api/auth/login` | Inicio de sesión |
| POST | `/api/auth/magic-link` | Magic link |
| GET | `/api/auth/current` | Usuario actual |

### Configuración

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/api/config/token` | Obtener webhook token |
| POST | `/api/config/token` | Actualizar webhook token |

### Ejemplos

```bash
# Sincronizar producto desde Odoo
curl -X POST http://localhost:5150/api/webhooks/odoo/update \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"id": 123, "name": "Producto", "list_price": 29.99, "default_code": "REF-001"}'

# Agregar al carrito
curl -X POST http://localhost:5150/api/carts/ -d '{"product_id": 123}'

# Registrar usuario
curl -X POST http://localhost:5150/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"name": "Usuario", "email": "user@example.com", "password": "secret"}'

# Checkout
curl -X POST http://localhost:5150/api/checkout \
  -H "Content-Type: application/json" \
  -d '{"customer": {"name": "Juan", "email": "juan@example.com", "phone": "555-0100", "street": "Calle 123", "city": "CIDMX"}, "cart_id": "<uuid>"}'
```

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

## Contribuir

1. Asegúrate de que `cargo test --all-features --all` pase.
2. Mantén el estilo: `cargo fmt --all` y `cargo clippy --all-features` sin warnings.
3. Usa `i32` para IDs de Odoo, `Uuid` para IDs internos.
4. PRs a la rama `main`. CI valida formato, linter y tests.

## Licencia

[LGPL-3.0](./LICENSE).
