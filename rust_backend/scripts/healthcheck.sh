#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0

check() {
    local desc="$1" method="$2" url="$3" expect="$4"
    shift 4
    local code
    code=$(curl -s -o /dev/null -w "%{http_code}" "$@" "$url" 2>/dev/null || echo "000")
    if [ "$code" = "$expect" ]; then
        echo -e "  ${GREEN}✓${NC} $desc ($code)"
        pass=$((pass+1))
    else
        echo -e "  ${RED}✗${NC} $desc → esperaba $expect, obtuvo $code"
        fail=$((fail+1))
    fi
}

check_json() {
    local desc="$1" url="$2" pattern="$3"
    local body
    body=$(curl -s "$url" 2>/dev/null || echo "")
    if echo "$body" | grep -q "$pattern"; then
        echo -e "  ${GREEN}✓${NC} $desc"
        pass=$((pass+1))
    else
        echo -e "  ${RED}✗${NC} $desc → no contiene '$pattern'"
        echo "    respuesta: $(echo "$body" | head -c 200)"
        fail=$((fail+1))
    fi
}

echo ""
echo "═══════════════════════════════════════"
echo "  Healthcheck: Odoo + Rust Loco Shop"
echo "═══════════════════════════════════════"
echo ""

# -- Odoo --
echo -e "${YELLOW}Odoo${NC}"
check "Página principal"    GET "http://localhost:8164/"              200
check "Web login"           GET "http://localhost:8164/web/login"     200

# -- Rust Loco Backend --
echo -e "\n${YELLOW}Rust Loco Backend${NC}"
check "Home page"           GET "http://localhost:5150/"               200
check "Catálogo"            GET "http://localhost:5150/shop/home"      200
check "Checkout"            GET "http://localhost:5150/checkout"       200
check "Search page"         GET "http://localhost:5150/shop/search"    200

# -- APIs --
echo -e "\n${YELLOW}APIs${NC}"
check "Product detail"      GET "http://localhost:5150/shop/api/product/1"  200

# Cart items require active guest cookie
check "Cart items (sin cookie)" GET "http://localhost:5150/api/carts/items" 404

# Shipping estimate
check_json "Shipping estimate" \
    "http://localhost:5150/api/shipping/estimate?product_ids=1&country=Mexico&state=CDMX" \
    "shipping_cost"

# -- Auth --
echo -e "\n${YELLOW}Auth${NC}"
check "Register page"       GET "http://localhost:5150/register"      200
check "Login page"          GET "http://localhost:5150/login"         200
check "Magic link page"     GET "http://localhost:5150/magic-link"    200

# -- Auth-protected (sin token) --
echo -e "\n${YELLOW}Auth-protected (sin token)${NC}"
check "Admin dashboard"     GET "http://localhost:5150/admin/dashboard"  401
check "Config page"         GET "http://localhost:5150/ui/auth/config"   403

# -- Static assets --
echo -e "\n${YELLOW}Static assets${NC}"
check "Tailwind CSS"       GET "http://localhost:5150/static/css/tailwind.css"       200
check "Checkout JS"        GET "http://localhost:5150/static/js/checkout.js"         200
check "HTMX"               GET "http://localhost:5150/static/js/htmx.min.js"         200

# -- Resumen --
echo ""
echo "═══════════════════════════════════════"
echo -e "Resultados: ${GREEN}$pass passed${NC}, ${RED}$fail failed${NC}"
echo "═══════════════════════════════════════"
echo ""
