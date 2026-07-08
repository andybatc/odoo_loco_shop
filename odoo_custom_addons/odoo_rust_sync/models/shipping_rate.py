from odoo import fields, models


class ShippingRate(models.Model):
    _name = "shipping.rate"
    _description = "Shipping Rate by State Pair"
    _rec_name = "display_name"

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

    def display_name(self):
        for r in self:
            r.display_name = f"{r.origin_state_id.name} → {r.dest_state_id.name}: ${r.amount:.2f}"
