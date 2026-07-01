# Setup Guide

## Prerequisites

- Docker & Docker Compose (recomendado), o
- Rust 2021 edition + Cargo, PostgreSQL 13+, Redis 6+

## Docker (recomendado)

```bash
git clone git@github.com:andybatc/odoo_loco_shop.git
cd odoo_loco_shop
cp .env.example .env

# Solo backend (shop funcional sin productos)
docker compose up

# Con Odoo incluido (catálogo completo)
docker compose --profile full up
```

Servicios:
| Servicio | Puerto | URL |
|----------|--------|-----|
| Rust Backend | 5150 | http://localhost:5150 |
| Swagger UI | 5150 | http://localhost:5150/swagger-ui |
| PostgreSQL | 5432 | local |
| Redis | 6379 | local |
| Odoo | 8069 | http://localhost:8069 |

La primera vez que arranca con `--profile full`, Odoo crea automáticamente la base de datos e instala los módulos (`muk_web_theme`, `odoo_rust_sync`, etc.).

## Manual

### Backend Rust

```bash
# PostgreSQL y Redis deben estar corriendo

cd rust_backend
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/odoo_shop_development"
export REDIS_URL="redis://127.0.0.1"
cargo loco start
```

El servidor arranca en `http://localhost:5150`. Las migraciones se ejecutan automáticamente (`auto_migrate: true`).

### Odoo

Requiere Odoo 18.0 Community o Enterprise con los módulos `sale` y `account` instalados.

```bash
cd /opt/odoo/custom_addons
ln -s /ruta/a/odoo_loco_shop/odoo_custom_addons/odoo_rust_sync .
ln -s /ruta/a/odoo_loco_shop/odoo_custom_addons/muk_web_* .

./odoo-bin --addons-path=/opt/odoo/custom_addons,/opt/odoo/addons \
           -d odoo_prod \
           -i odoo_rust_sync,muk_web_theme
```

## Post-installation

1. Configurar webhook Odoo → Rust (ver [Odoo Integration](./Odoo-Integration))
2. Poblar productos (crear en Odoo o usar sync masivo)
3. Opcional: registrar usuario administrador en `http://localhost:5150/ui/auth/register`

### Sync masivo de productos

Si ya existe una base de datos Odoo con productos:

```bash
cd rust_backend
ODOO_DATABASE_URL="postgres://...@.../odoo_prod" cargo loco task sync
```
