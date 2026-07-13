from odoo import fields, models, api
import logging
import requests

_logger = logging.getLogger(__name__)


class ShippingRate(models.Model):
    _name = "shipping.rate"
    _description = "Shipping Rate by State Pair"

    origin_country_id = fields.Many2one("res.country", string="País origen", required=True)
    origin_state_id = fields.Many2one("res.country.state", string="Estado origen", required=True)
    dest_country_id = fields.Many2one("res.country", string="País destino", required=True)
    dest_state_id = fields.Many2one("res.country.state", string="Estado destino", required=True)
    amount = fields.Float(string="Costo de envío", required=True)

    _sql_constraints = [
        (
            "unique_route",
            "unique(origin_country_id, origin_state_id, dest_country_id, dest_state_id)",
            "Ya existe una tarifa para esta ruta",
        ),
    ]

    def _compute_display_name(self):
        for r in self:
            r.display_name = f"{r.origin_state_id.name} → {r.dest_state_id.name}: ${r.amount:.2f}"

    def _sync_to_shop(self):
        shop_url = self.env["ir.config_parameter"].sudo().get_param(
            "rust_shop_url", "http://localhost:5150"
        )
        token = self.env["ir.config_parameter"].sudo().get_param("rust_shop_token", "")
        if not token:
            _logger.warning("rust_shop_token no configurado, saltando sync de tarifas")
            return
        rates_data = [
            {
                "origin_country": r.origin_country_id.with_context(lang="en_US").name,
                "origin_state": r.origin_state_id.name,
                "dest_country": r.dest_country_id.with_context(lang="en_US").name,
                "dest_state": r.dest_state_id.name,
                "amount": r.amount,
            }
            for r in self
        ]
        try:
            resp = requests.post(
                f"{shop_url}/api/shipping/rates/sync",
                headers={
                    "Authorization": f"Bearer {token}",
                    "Content-Type": "application/json",
                },
                json={"rates": rates_data},
                timeout=10,
            )
            if resp.status_code not in (200, 202):
                _logger.error(
                    "Error sync tarifas: HTTP %s - %s", resp.status_code, resp.text[:200]
                )
            else:
                _logger.info("Tarifas sincronizadas con la tienda (%d registros)", len(rates_data))
        except Exception as e:
            _logger.error("Error conectando con la tienda: %s", e)

    @api.model_create_multi
    def create(self, vals_list):
        records = super().create(vals_list)
        records._sync_to_shop()
        return records

    def write(self, vals):
        result = super().write(vals)
        if result:
            self._sync_to_shop()
        return result
