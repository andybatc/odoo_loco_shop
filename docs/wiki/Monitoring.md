# Monitoreo — Prometheus + Grafana

## Stack

| Component | Puerto | Imagen |
|-----------|--------|--------|
| **Prometheus** | `:9090` | `prom/prometheus:latest` |
| **Grafana** | `:3000` | `grafana/grafana:latest` |
| **Rust backend** | `:5150/metrics` | nativo (`axum-prometheus`) |
| **Odoo** | `:8069/metrics` | `odoo_rust_sync` controller |

## Arquitectura

```
Rust Backend ──GET /metrics──▶ Prometheus ◀── Grafana
       │                          │
       │                          │
Odoo ──GET /metrics───────────────┘
```

- Prometheus scrapea los endpoints `/metrics` de Rust y Odoo cada 15s
- Grafana consulta Prometheus como datasource
- Dashboards se provisionan automáticamente desde `monitoring/grafana/dashboards/`

## Cómo levantar

```bash
# Todo el stack completo (incluye Odoo + monitoreo):
docker compose --profile full up -d

# Solo monitoreo (sin Odoo):
docker compose up -d prometheus grafana
```

### Acceso

| Servicio | URL | Credenciales |
|----------|-----|-------------|
| **Prometheus** | http://localhost:9090 | — |
| **Grafana** | http://localhost:3000 | `admin` / `admin` |
| **Rust /metrics** | http://localhost:5150/metrics | — |
| **Odoo /metrics** | http://localhost:8069/metrics | — |

## Endpoints de métricas

### Rust backend (`/metrics`)

Expone automáticamente vía `axum-prometheus` 0.9:

- `http_requests_total` — total de requests HTTP (labels: method, path, status)
- `http_request_duration_seconds` — histograma de duración
- Métricas Go-style del runtime

### Odoo (`/metrics`)

Expone vía el módulo `odoo_rust_sync` usando `prometheus-client` Python:

- `odoo_http_requests_total` — total de requests HTTP a Odoo
- `odoo_users_total` — usuarios activos (gauge)
- `odoo_products_total` — productos publicados (gauge)
- `odoo_sale_orders_total` — órdenes de venta creadas (counter)
- `odoo_sync_duration_seconds` — duración de sincronizaciones (histogram)

## Dashboards

Se provisiona automáticamente `Odoo Shop` en Grafana con paneles:

| Panel | Fuente | Descripción |
|-------|--------|-------------|
| HTTP Requests Rate | Rust | requests/s por método+ruta |
| HTTP Error Rate | Rust | requests/s con status 5xx |
| HTTP Duration (p50/p95) | Rust | latencia percentiles |
| Active Users | Odoo | usuarios activos total |
| Published Products | Odoo | productos publicados total |
| Sync Duration | Odoo | duración promedio de sync |

## Agregar métricas custom

### Rust backend

En cualquier handler, usar `axum_prometheus`:

```rust
use axum_prometheus::metrics::counter;
let c = counter!("my_custom_metric", "description");
c.increment(1);
```

### Odoo

En `controllers/prometheus_metrics.py`, declarar nuevos contadores/gauges:

```python
from prometheus_client import Counter
MY_METRIC = Counter("odoo_my_metric_total", "description")
MY_METRIC.inc()
```

## Config

Archivos bajo `monitoring/`:

```
monitoring/
├── prometheus.yml                          # Config de scrape
└── grafana/
    ├── datasources/
    │   └── datasources.yml                 # Datasource Prometheus provisionado
    └── dashboards/
        ├── dashboards.yml                  # Provisioner de dashboards
        └── odoo_shop.json                  # Dashboard Odoo Shop
```
