# Monitoreo — Prometheus + Grafana

## Stack

| Componente | Puerto | Tipo |
|-----------|--------|------|
| **node-exporter** | `:9100/metrics` | sistema (nativo) |
| **postgres-exporter** | `:9187/metrics` | sistema (nativo) |
| **redis-exporter** | `:9121/metrics` | sistema (nativo) |
| **Prometheus** | `:9090` | nativo o Docker |
| **Grafana** | `:3000` | nativo o Docker |
| **Rust backend** | `:5150/metrics` | nativo (`axum-prometheus`) |
| **Odoo** | `:8072/metrics` | nativo (`odoo_rust_sync` controller) |
| **process-exporter** | `:9256/metrics` | nativo o Docker |

## Arquitectura

```
Rust Backend ──GET /metrics─────┐
Odoo ─────────GET /metrics─────┤
node-exporter ─────────────────┤
postgres-exporter ─────────────┤──▶ Prometheus ◀── Grafana
redis-exporter ────────────────┤       │
process-exporter ──────────────┘       │
                                  │
                        (scrapea cada 15s)
```

- Prometheus scrapea los endpoints `/metrics` de cada servicio cada 15s
- Grafana consulta Prometheus como datasource
- Dashboards se provisionan automáticamente desde archivos

## Cómo levantar

### Opción A: Nativo (systemd)

Requiere Prometheus y Grafana instalados como servicios del sistema.

```bash
# Setup completo (configs + process-exporter):
./scripts/setup-monitoring.sh
```

O manualmente:

```bash
# 1. Copiar config de Prometheus
sudo cp monitoring/prometheus.yml /etc/prometheus/prometheus.yml
sudo killall -HUP prometheus

# 2. process-exporter
sudo cp monitoring/process-exporter.yml /etc/prometheus/
# Descargar de https://github.com/ncabatoff/process-exporter/releases
sudo cp process-exporter /usr/local/bin/
# Crear servicio systemd (ver setup-monitoring.sh)

# 3. Grafana provisioning
sudo cp -r monitoring/grafana/provisioning/* /etc/grafana/provisioning/
sudo cp monitoring/grafana/dashboards/odoo_shop.json /etc/grafana/provisioning/dashboards/
sudo systemctl restart grafana-server
```

### Opción B: Docker Compose

```bash
# Iniciar stack de monitoreo
docker compose up -d

# Verificar
docker compose ps
curl -s http://localhost:9090/api/v1/targets | python3 -c "
import sys,json
for t in json.load(sys.stdin)['data']['activeTargets']:
    print(t['scrapeUrl'], chr(8594), t['health'])
"
```

Nota: si venías usando servicios nativos, pararlos primero:
```bash
sudo systemctl stop prometheus grafana-server process-exporter
sudo systemctl disable prometheus grafana-server process-exporter
```

### Acceso

| Servicio | URL | Credenciales |
|----------|-----|-------------|
| **Prometheus** | http://localhost:9090 | — |
| **Grafana** | http://localhost:3000 | `admin` / `admin` |
| **Rust /metrics** | http://localhost:5150/metrics | — |
| **Odoo /metrics** | http://localhost:8072/metrics | — |
| **process-exporter** | http://localhost:9256/metrics | — |

## Métricas expuestas

### Rust backend (`axum-prometheus`)

Nombres con prefijo `axum_`:

| Métrica | Tipo | Labels |
|---------|------|--------|
| `axum_http_requests_total` | counter | method, status, endpoint |
| `axum_http_requests_duration_seconds` | histogram | method, status, endpoint |

### Odoo (`prometheus-client`)

| Métrica | Tipo | Descripción |
|---------|------|-------------|
| `odoo_http_requests_total` | counter | Requests HTTP (method, endpoint) |
| `odoo_db_connections` | gauge | Conexiones actuales a DB |
| `odoo_users_total` | gauge | Usuarios activos |
| `odoo_products_total` | gauge | Productos publicados (`sale_ok = True`) |
| `odoo_products_unpublished` | gauge | Productos no publicados (`sale_ok = False`) |
| `odoo_products_no_stock` | gauge | Productos con stock ≤ 0 |
| `odoo_products_low_stock` | gauge | Productos con stock entre 1 y 10 |
| `odoo_sale_orders_total` | counter | Órdenes de venta creadas |
| `odoo_sync_duration_seconds` | histogram | Duración de sincronizaciones |

### system (node-exporter)

Métricas estándar de sistema: CPU, memoria, disco, red, load.

### Por proceso (process-exporter)

| Métrica | Descripción |
|---------|-------------|
| `namedprocess_namegroup_cpu_seconds_total` | CPU por proceso (rust, odoo, postgres, redis) |
| `namedprocess_namegroup_memory_bytes` | Memoria RSS por proceso |
| `namedprocess_namegroup_num_procs` | Cantidad de procesos |
| `namedprocess_namegroup_open_filedesc` | Archivos abiertos |

## Dashboard

Se provisiona automáticamente **Odoo Shop** en Grafana con 13 paneles:

| # | Panel | Tipo | Fuente |
|---|-------|------|--------|
| 1 | CPU | gauge | node-exporter |
| 2 | Memory | gauge | node-exporter |
| 3 | Load Average | timeseries | node-exporter |
| 4 | CPU por proceso | timeseries | process-exporter |
| 5 | Memoria por proceso (RSS) | timeseries | process-exporter |
| 6 | HTTP Requests Rate | timeseries | Rust (`axum_`) |
| 7 | HTTP Error Rate (5xx) | timeseries | Rust (`axum_`) |
| 8 | HTTP Duration (p50/p95) | timeseries | Rust (`axum_`) |
| 9 | Active Users | stat | Odoo |
| 10 | Published Products | stat | Odoo |
| 11 | Unpublished | stat | Odoo |
| 12 | No Stock | stat | Odoo |
| 13 | Low Stock (≤10) | stat | Odoo |
| 14 | Sync Duration | timeseries | Odoo |

## Agregar métricas custom

### Rust backend

```rust
use axum_prometheus::metrics::counter;
let c = counter!("axum_mi_metrica_total", "descripción");
c.increment(1);
```

### Odoo

En `odoo_custom_addons/odoo_rust_sync/controllers/prometheus_metrics.py`:

```python
from prometheus_client import Gauge
MI_METRICA = Gauge("odoo_mi_metrica", "descripción")
MI_METRICA.set(valor)
```

## Archivos

```
monitoring/
├── prometheus.yml                          # Targets de scrape
├── process-exporter.yml                    # Regex de procesos a monitorear
└── grafana/
    ├── provisioning/
    │   ├── datasources/datasources.yml     # Datasource Prometheus
    │   └── dashboards/dashboards.yml       # Provider de dashboards
    └── dashboards/
        └── odoo_shop.json                  # Dashboard Odoo Shop

scripts/
└── setup-monitoring.sh                     # Setup nativo completo

docker-compose.yml                          # Stack monitoreo en Docker
```

## Requisitos

- Prometheus instalado (`apt install prometheus` o Docker)
- Grafana instalado (`apt install grafana` o Docker)
- Odoo corriendo en puerto `:8072` con addons path que incluya `odoo_custom_addons`
- Rust backend exponiendo `:5150/metrics`
- Postgres, Redis, node-exporter, postgres-exporter, redis-exporter funcionando
