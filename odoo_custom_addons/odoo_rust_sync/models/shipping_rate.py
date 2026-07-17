from odoo import fields, models, api
import logging

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
        """Enqueue webhook instead of direct sync"""
        token = self.env["ir.config_parameter"].sudo().get_param("rust_shop_token", "")
        if not token:
            _logger.warning("rust_shop_token no configurado, saltando sync de tarifas")
            return

        for record in self:
            payload = {
                "rates": [{
                    "origin_country": record.origin_country_id.with_context(lang="en_US").name,
                    "origin_state": record.origin_state_id.name,
                    "dest_country": record.dest_country_id.with_context(lang="en_US").name,
                    "dest_state": record.dest_state_id.name,
                    "amount": record.amount,
                }]
            }
            self.env['rust_webhook.queue'].enqueue(
                model_name='shipping.rate',
                res_id=record.id,
                webhook_type='shipping_sync',
                payload=payload,
            )

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
