# Odoo Loco Shop

Catálogo web de alto rendimiento con Odoo 18 como ERP y un frontend/backend desacoplado en Rust (Loco.rs) con Redis para caché y colas.

## Quick Links

- [Setup Guide](./Setup)
- [Architecture](./Architecture)
- [Odoo Integration](./Odoo-Integration)
- [Development](./Development)
- [API Reference](./API)
- [Monitoring](./Monitoring)

## Stack

| Componente | Tecnología |
|------------|-----------|
| Backend web | Rust 2021, Loco.rs 0.16 (Axum 0.8), SeaORM 1.1 |
| Frontend | Vue 3 (global build), Tailwind CSS, Tera templates |
| ERP | Odoo 18.0 (Python 3.12) |
| Base de datos | PostgreSQL 13+ |
| Cache / Colas | Redis 6+ |
| CI | GitHub Actions (rustfmt + clippy + tests) |

## Repositorio

```bash
git clone git@github.com:andybatc/odoo_loco_shop.git
cd odoo_loco_shop
cp .env.example .env
docker compose up
```
