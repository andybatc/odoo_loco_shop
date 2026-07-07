#!/usr/bin/env bash
set -euo pipefail

# Setup monitoring for Odoo Shop (Prometheus + Grafana + exporters)
# Run from repo root: ./scripts/setup-monitoring.sh

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Copying Prometheus config"
sudo cp "$REPO_ROOT/monitoring/prometheus.yml" /etc/prometheus/prometheus.yml
sudo killall -HUP prometheus 2>/dev/null || sudo systemctl restart prometheus

echo "==> Copying process-exporter config"
sudo mkdir -p /etc/prometheus
sudo cp "$REPO_ROOT/monitoring/process-exporter.yml" /etc/prometheus/process-exporter.yml

if [ ! -f /usr/local/bin/process-exporter ]; then
    echo "==> Installing process-exporter"
    cd /tmp
    curl -sL https://github.com/ncabatoff/process-exporter/releases/download/v0.8.7/process-exporter-0.8.7.linux-amd64.tar.gz | tar xz
    sudo cp process-exporter-0.8.7.linux-amd64/process-exporter /usr/local/bin/
    rm -rf process-exporter-0.8.7.linux-amd64
    cd "$REPO_ROOT"
fi

if [ ! -f /etc/systemd/system/process-exporter.service ]; then
    echo "==> Creating process-exporter systemd service"
    sudo tee /etc/systemd/system/process-exporter.service > /dev/null <<'EOF'
[Unit]
Description=Process Exporter
After=network.target

[Service]
ExecStart=/usr/local/bin/process-exporter --config.path=/etc/prometheus/process-exporter.yml
Restart=always
User=root

[Install]
WantedBy=multi-user.target
EOF
    sudo systemctl daemon-reload
fi
sudo systemctl enable --now process-exporter

echo "==> Copying Grafana provisioning"
sudo mkdir -p /etc/grafana/provisioning/dashboards /etc/grafana/provisioning/datasources
sudo cp "$REPO_ROOT/monitoring/grafana/provisioning/dashboards/dashboards.yml" /etc/grafana/provisioning/dashboards/
sudo cp "$REPO_ROOT/monitoring/grafana/provisioning/datasources/datasources.yml" /etc/grafana/provisioning/datasources/
sudo cp "$REPO_ROOT/monitoring/grafana/dashboards/odoo_shop.json" /etc/grafana/provisioning/dashboards/
sudo systemctl enable --now grafana-server 2>/dev/null || true

echo ""
echo "==> Done. Verifying targets..."
sleep 5
curl -s http://localhost:9090/api/v1/targets | python3 -c "
import sys,json
for t in json.load(sys.stdin)['data']['activeTargets']:
    print(f'  {t[\"scrapeUrl\"]:40} {t[\"health\"]}')
"
echo ""
echo "Grafana: http://localhost:3000 (admin / admin)"
echo "Dashboard: http://localhost:3000/d/odoo_shop/odoo-shop"
echo "Prometheus: http://localhost:9090"
