# Containerization & K8s Deployment Design

## Stack

- **Rust backend** (Loco.rs 0.16, Axum 0.8)
- **PostgreSQL 16** (database)
- **Redis 7** (cache + queue)
- **Prometheus + Grafana** (monitoring)
- **Odoo 18.0** stays external (not containerized)

## Local Development — docker-compose.yml

Single command: `docker compose up -d`

Services:
| Service | Image | Port |
|---------|-------|------|
| postgres | postgres:16-alpine | - |
| redis | redis:7-alpine | - |
| backend | build: ./rust_backend | 5150 |
| prometheus | prom/prometheus:latest | 9090 |
| grafana | grafana/grafana:latest | 3000 |

Backend env vars from compose environment (DB URL, Redis URL). Secrets use `${DB_PASSWORD:-dev_password}` pattern.

## Dockerfile (rust_backend/Dockerfile)

Three-stage:
1. **planner** — cache Rust dependencies layer (faster rebuilds)
2. **builder** — compile with `--frozen`, includes `.patches/` and `.cargo/config.toml`
3. **runtime** — debian:bookworm-slim, runs `./odoo_shop-cli start --server-and-worker`

## K8s — Kustomize layout

```
k8s/
  base/
    kustomization.yaml          # wires all resources
    namespace.yaml              # odoo-shop
    configmap.yaml              # app config (non-sensitive)
    secrets.yaml                # template with placeholder values
    backend-deployment.yaml     # app server
    backend-service.yaml        # ClusterIP :5150
    postgres-statefulset.yaml   # DB with PVC (10Gi)
    postgres-service.yaml       # ClusterIP :5432
    redis-deployment.yaml       # cache/queue
    redis-service.yaml          # ClusterIP :6379
    prometheus.yaml             # ConfigMap + Deployment + Service
    grafana.yaml                # ConfigMap + Deployment + Service
  overlays/
    local/
      kustomization.yaml        # 1 replica, no resource limits
    production/
      kustomization.yaml        # 3 backend replicas, resource limits, probes
```

Secrets managed as plain template (`secrets.yaml` with placeholder values). For production, replace values manually or use Sealed Secrets / External Secrets operator.

Ingress defined in `base/` with host as placeholder; `overlays/production/` patches the real domain via strategic merge.

## Configs

- `backend-config` ConfigMap: mounts `config/production.yaml` (or `development.yaml` for local)
- `backend-secrets` Secret: `DATABASE_URL`, `REDIS_URL`, `JWT_SECRET`, `WEBHOOK_TOKEN`
- `postgres` Secret: `POSTGRES_PASSWORD`

## Commands

```bash
# Local dev (docker)
docker compose up -d

# Local dev (k8s/minikube)
kubectl apply -k k8s/overlays/local

# Production
kubectl apply -k k8s/overlays/production

# Build image
docker build -t odoo-shop-backend:latest ./rust_backend
```
