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
            ODOO_PRODUCTS_TOTAL.set(
                request.env["product.template"]
                .sudo()
                .search_count([("sale_ok", "=", True)])
            )
        except Exception:
            pass

        data = generate_latest(REGISTRY)
        return http.Response(
            data,
            content_type="text/plain; version=0.0.4",
            headers={"X-Robots-Tag": "noindex"},
        )
