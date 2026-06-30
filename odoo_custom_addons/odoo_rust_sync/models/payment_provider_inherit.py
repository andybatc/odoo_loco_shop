from odoo import models, fields, api
from odoo.exceptions import UserError
import requests
import logging

_logger = logging.getLogger(__name__)


class PaymentProvider(models.Model):
    _inherit = 'payment.provider'

    def write(self, vals):
        res = super(PaymentProvider, self).write(vals)
        sync_fields = ['state', 'name', 'is_published']
        if any(f in vals for f in sync_fields):
            for record in self:
                record._send_rust_webhook()
        return res

    @api.model_create_multi
    def create(self, vals_list):
        records = super(PaymentProvider, self).create(vals_list)
        for record in records:
            if record.state != 'disabled':
                record._send_rust_webhook()
        return records

    def _to_rust_payload(self):
        return {
            "odoo_provider_id": self.id,
            "name": self.name,
            "code": self.code,
            "state": self.state,
            "is_published": self.is_published,
            "allow_tokenization": self.allow_tokenization,
            "capture_manually": self.capture_manually,
            "sequence": self.sequence,
        }

    def _get_rust_base_url(self):
        return self.env['ir.config_parameter'].sudo().get_param(
            'rust_api.base_url',
            default='http://127.0.0.1:5150'
        ).rstrip('/')

    def _send_rust_webhook(self):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token:
            _logger.warning("No webhook_token configurado, omitiendo sync de payment provider")
            return

        base_url = self._get_rust_base_url()
        url = f"{base_url}/api/webhooks/odoo/payment-methods"
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }

        payload = [self._to_rust_payload()]

        def send_after_commit():
            try:
                response = requests.post(url, json=payload, headers=headers, timeout=5)
                if response.status_code != 200:
                    _logger.warning("Rust respondio %s al sync de payment provider", response.status_code)
                else:
                    _logger.info("Payment provider %s sincronizado con Rust", self.name)
            except Exception as e:
                _logger.error("Error sync payment provider: %s", str(e))

        self.env.cr.postcommit.add(send_after_commit)

    def action_sync_payment_methods(self):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token:
            raise UserError("El webhook_token no esta configurado en Parametros del Sistema.")

        base_url = self._get_rust_base_url()
        url = f"{base_url}/api/webhooks/odoo/payment-methods"
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }

        batch = [rec._to_rust_payload() for rec in self]

        try:
            response = requests.post(url, json=batch, headers=headers, timeout=15)
            if response.status_code == 200:
                return {
                    'type': 'ir.actions.client',
                    'tag': 'display_notification',
                    'params': {
                        'title': 'Sincronizacion exitosa',
                        'message': 'Se sincronizaron %d metodo(s) de pago con la tienda.' % len(batch),
                        'type': 'success',
                        'sticky': False,
                    }
                }
            else:
                raise UserError("Rust respondio con error: %d" % response.status_code)
        except Exception as e:
            raise UserError("Error de conexion con Rust: %s" % str(e))
