from odoo import models, fields, api
from odoo.exceptions import UserError
import requests
import logging

_logger = logging.getLogger(__name__)


class ProductTemplate(models.Model):
    _inherit = 'product.template'

    warehouse_country_id = fields.Many2one("res.country", string="País del almacén")
    warehouse_state_id = fields.Many2one("res.country.state", string="Estado del almacén")
    warehouse_latitude = fields.Float(string="Latitud del almacén", digits=(9, 6))
    warehouse_longitude = fields.Float(string="Longitud del almacén", digits=(9, 6))

    @api.onchange('warehouse_country_id', 'warehouse_state_id')
    def _onchange_warehouse_location(self):
        if self.warehouse_country_id and self.warehouse_state_id:
            try:
                q = f"{self.warehouse_state_id.name}, {self.warehouse_country_id.name}"
                resp = requests.get(
                    "https://nominatim.openstreetmap.org/search",
                    params={"q": q, "format": "json", "limit": 1},
                    headers={"User-Agent": "Odoo/odoo_rust_sync/1.0"},
                    timeout=3,
                )
                data = resp.json()
                if data:
                    self.warehouse_latitude = float(data[0]["lat"])
                    self.warehouse_longitude = float(data[0]["lon"])
            except Exception:
                pass  # falla silencioso, coordenadas se quedan vacías

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
            "is_published"
        ]
        if any(f in vals for f in sync_fields):
            for record in self:
                record._send_rust_webhook()
        return res

    def _get_tax_percent(self):
        """ Devuelve el porcentaje de impuesto del producto (primera tasa de venta) """
        tax = self.taxes_id[:1]
        return float(tax.amount) if tax else 0.0

    def _to_rust_payload(self):
        """ Devuelve el diccionario formateado listo para el webhook """
        name_field = self.name
        product_name = name_field.get('es_ES') or name_field.get('en_US') or list(name_field.values())[0] if isinstance(
            name_field, dict) else name_field
        return {
            "odoo_id": self.id,
            "name": product_name or "Sin nombre",
            "price": float(self.list_price),
            "image_base64": self.image_1920.decode('utf-8') if self.image_1920 else None,
            "is_published": self.is_published,
            "tax_percent": self._get_tax_percent(),
        }

    def _get_rust_base_url(self):
        return self.env['ir.config_parameter'].sudo().get_param(
            'rust_api.base_url',
            default='http://127.0.0.1:5150'
        ).rstrip('/')

    def _send_rust_webhook(self):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token:
            _logger.warning("⚠️ No se envió el webhook a Rust porque 'rust_api.webhook_token' no está configurado.")
            return

        base_url = self._get_rust_base_url()
        url = f"{base_url}/api/webhooks/odoo/update"
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }

        payload = self._to_rust_payload()

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

    def action_open_google_maps(self):
        self.ensure_one()
        parts = []
        if self.warehouse_country_id:
            parts.append(self.warehouse_country_id.name)
        if self.warehouse_state_id:
            parts.append(self.warehouse_state_id.name)
        if not parts:
            raise UserError("Selecciona un país o estado primero para abrir el mapa.")

        url = f"https://www.google.com/maps/search/{'+'.join(parts)}"
        return {
            'type': 'ir.actions.act_url',
            'url': url,
            'target': 'new',
        }

    def _to_bulk_payload(self):
        """ Payload ligero para sincronización masiva (sin imagen) """
        name_field = self.name
        product_name = name_field.get('es_ES') or name_field.get('en_US') or list(name_field.values())[0] if isinstance(
            name_field, dict) else name_field
        return {
            "odoo_id": self.id,
            "name": product_name or "Sin nombre",
            "price": float(self.list_price),
            "is_published": self.is_published,
            "warehouse_country": self.warehouse_country_id.name if self.warehouse_country_id else None,
            "warehouse_state": self.warehouse_state_id.name if self.warehouse_state_id else None,
            "tax_percent": self._get_tax_percent(),
        }

    def action_bulk_sync_to_rust(self):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token:
            raise UserError("⚠️ El token 'rust_api.webhook_token' no está configurado en los Parámetros del Sistema.")

        base_url = self._get_rust_base_url()
        url = f"{base_url}/api/webhooks/odoo/bulk-update"

        batch_products = [rec._to_bulk_payload() for rec in self]

        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }

        try:
            response = requests.post(url, json=batch_products, headers=headers, timeout=15)

            if response.status_code == 200:
                # Devolvemos la notificación flotante para que la UI de Odoo la dibuje
                return {
                    'type': 'ir.actions.client',
                    'tag': 'display_notification',
                    'params': {
                        'title': 'Sincronización en proceso',
                        'message': f'Se enviaron {len(batch_products)} productos exitosamente a la tienda.',
                        'type': 'success',
                        'sticky': False,
                    }
                }
            else:
                raise UserError(f"El backend de Rust respondió con código de error: {response.status_code}")

        except Exception as e:
            raise UserError(f"❌ Fallo crítico de conexión con Rust Backend: {str(e)}")
