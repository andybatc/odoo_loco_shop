import logging

from odoo import http
from odoo.http import request

_logger = logging.getLogger(__name__)

try:
    from prometheus_client import generate_latest, REGISTRY, Counter, Gauge, Histogram

    ODOO_HTTP_REQUESTS = Counter(
        "odoo_http_requests_total", "Total HTTP requests", ["method", "endpoint"]
    )
    ODOO_DB_CONNECTIONS = Gauge("odoo_db_connections", "Current DB connections")
    ODOO_USERS_TOTAL = Gauge("odoo_users_total", "Total active users")
    ODOO_PRODUCTS_TOTAL = Gauge("odoo_products_total", "Total published products")
    ODOO_PRODUCTS_UNPUBLISHED = Gauge("odoo_products_unpublished", "Total unpublished products")
    ODOO_PRODUCTS_NO_STOCK = Gauge("odoo_products_no_stock", "Products with zero stock")
    ODOO_PRODUCTS_LOW_STOCK = Gauge("odoo_products_low_stock", "Products with low stock (1-10 units)")
    ODOO_SALE_ORDERS = Counter(
        "odoo_sale_orders_total", "Total sale orders created"
    )
    ODOO_SYNC_DURATION = Histogram(
        "odoo_sync_duration_seconds", "Duration of sync operations",
        buckets=[0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0],
    )
    PROMETHEUS_AVAILABLE = True
except ImportError:
    _logger.warning("prometheus_client not installed — Odoo metrics disabled")
    PROMETHEUS_AVAILABLE = False


class PrometheusMetricsController(http.Controller):

    @http.route("/metrics", type="http", auth="public", csrf=False)
    def metrics(self):
        if not PROMETHEUS_AVAILABLE:
            return http.Response(
                "# prometheus_client not installed\n",
                content_type="text/plain; version=0.0.4",
                status=503,
            )

        try:
            ODOO_USERS_TOTAL.set(
                request.env["res.users"].sudo().search_count([("active", "=", True)])
            )
        except Exception:
            pass

        try:
            products = request.env["product.template"].sudo()
            ODOO_PRODUCTS_TOTAL.set(products.search_count([("sale_ok", "=", True)]))
            ODOO_PRODUCTS_UNPUBLISHED.set(products.search_count([("sale_ok", "=", False)]))
        except Exception:
            pass

        try:
            product_variants = request.env["product.product"].sudo()
            ODOO_PRODUCTS_NO_STOCK.set(
                product_variants.search_count([("sale_ok", "=", True), ("qty_available", "<=", 0)])
            )
            ODOO_PRODUCTS_LOW_STOCK.set(
                product_variants.search_count(
                    [("sale_ok", "=", True), ("qty_available", ">", 0), ("qty_available", "<=", 10)]
                )
            )
        except Exception:
            pass

        data = generate_latest(REGISTRY)
        return http.Response(
            data,
            content_type="text/plain; version=0.0.4",
            headers={"X-Robots-Tag": "noindex"},
        )
