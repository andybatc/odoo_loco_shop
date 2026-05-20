from odoo import models, fields, api
import requests
import logging

_logger = logging.getLogger(__name__)


class ProductTemplate(models.Model):
    _inherit = 'product.template'

    @api.model_create_multi
    def create(self, vals_list):
        """ Se ejecuta al crear nuevos productos """
        # 1. Llamar al super para crear los registros en la DB y obtener sus IDs
        records = super(ProductTemplate, self).create(vals_list)

        # 2. Notificar a Rust por cada registro creado
        for record in records:
            record._send_rust_webhook()

        return records

    def write(self, vals):
        # 1. Ejecutar el guardado normal de Odoo
        res = super(ProductTemplate, self).write(vals)

        # 2. Si cambian campos clave, notificamos a Rust
        # Evitamos campos técnicos para no saturar el webhook
        sync_fields = [
            'name',
            'list_price',
            'default_code',
            'standard_price',
            "image_1920",
        ]
        if any(f in vals for f in sync_fields):
            for record in self:
                record._send_rust_webhook()
        return res

    def _send_rust_webhook(self):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token:
            _logger.warning("⚠️ No se envió el webhook a Rust porque 'rust_api.webhook_token' no está configurado.")
            return

        url = "http://127.0.0.1:5150/api/webhooks/odoo/update"
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }

        # Tratamiento del nombre (Odoo 18 usa traducción multilingüe en JSON/dict)
        name_field = self.name
        product_name = name_field.get('es_ES') or name_field.get('en_US') or list(name_field.values())[0] if isinstance(
            name_field, dict) else name_field

        # 🚀 Armamos el payload con la información completa del producto
        payload = {
            "odoo_id": self.id,
            "name": product_name or "Sin nombre",
            "price": float(self.list_price),
            "image_base64": self.image_1920.decode('utf-8') if self.image_1920 else None
        }

        _logger.info("🚀 [WEBHOOK ODOO] Datos preparados para enviar -> ID: %s, Name: %s, Price: %s",
                     payload['odoo_id'], payload['name'], payload['price'])

        def send_after_commit():
            try:
                response = requests.post(url, json=payload, headers=headers, timeout=5)
                if response.status_code != 200:
                    _logger.warning("⚠️ Rust respondió con código inesperado: %s", response.status_code)
            except Exception as e:
                _logger.error("❌ Fallo de conexión tardía con Rust Backend: %s", str(e))

        self.env.cr.postcommit.add(send_after_commit)
