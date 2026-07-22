#!/usr/bin/env bash
set -euo pipefail

# Tunnel para desarrollo local con Stripe Checkout.
# Uso: ./scripts/tunnel.sh [ngrok|stop]
#   ngrok (default):    Inicia ngrok, configura odoo_base_url automáticamente
#   stop:               Detiene ngrok

NGROK_PORT=5150
API_BASE="http://localhost:5150"
NGROK_PID_FILE="/tmp/ngrok_shop.pid"

start_tunnel() {
    if [ -f "$NGROK_PID_FILE" ] && kill -0 "$(cat "$NGROK_PID_FILE")" 2>/dev/null; then
        echo "ngrok ya está corriendo (PID $(cat "$NGROK_PID_FILE"))"
        exit 1
    fi

    echo "Iniciando ngrok en puerto $NGROK_PORT..."
    nohup ngrok http "$NGROK_PORT" --log=stdout > /tmp/ngrok_shop.log 2>&1 &
    NGROK_PID=$!
    echo "$NGROK_PID" > "$NGROK_PID_FILE"

    # Esperar a que ngrok esté listo y obtener la URL
    for i in $(seq 1 15); do
        sleep 1
        NGROK_URL=$(curl -s http://127.0.0.1:4040/api/tunnels | python3 -c "import sys,json; print(json.load(sys.stdin)['tunnels'][0]['public_url'])" 2>/dev/null || true)
        if [ -n "$NGROK_URL" ]; then
            break
        fi
    done

    if [ -z "$NGROK_URL" ]; then
        echo "ERROR: ngrok no respondió. Revisa /tmp/ngrok_shop.log"
        exit 1
    fi

    echo "Tunnel activo: $NGROK_URL"

    # Configurar odoo_base_url en el backend
    echo "Configurando odoo_base_url = $NGROK_URL"
    curl -s -X POST "$API_BASE/api/config/odoo-url" \
        -H "Content-Type: application/json" \
        -d "{\"url\": \"$NGROK_URL\"}" > /dev/null && echo "✓ URL configurada" || echo "✗ No se pudo configurar (¿backend corriendo?)"

    echo ""
    echo "ngrok dashboard: http://127.0.0.1:4040"
    echo "Stripe CLI (en otra terminal): stripe listen --forward-to localhost:5150/api/webhooks/stripe"
}

stop_tunnel() {
    if [ -f "$NGROK_PID_FILE" ]; then
        PID=$(cat "$NGROK_PID_FILE")
        kill "$PID" 2>/dev/null || true
        rm -f "$NGROK_PID_FILE"
        echo "ngrok detenido"
    else
        echo "ngrok no estaba corriendo"
    fi
}

case "${1:-ngrok}" in
    ngrok|start) start_tunnel ;;
    stop)        stop_tunnel  ;;
    *)           echo "Uso: $0 [ngrok|stop]" && exit 1 ;;
esac
