from odoo import models, fields, api
import secrets
import requests
import logging

_logger = logging.getLogger(__name__)


class ResConfigSettings(models.TransientModel):
    _inherit = 'res.config.settings'

    rust_shop_admin_id = fields.Many2one('res.users', string='Admin de la Tienda')

    def set_rust_webhook_token(self):
        existing_token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not existing_token:
            new_token = secrets.token_urlsafe(32)
            self.env['ir.config_parameter'].sudo().set_param('rust_api.webhook_token', new_token)

    def set_rust_base_url(self):
        existing_url = self.env['ir.config_parameter'].sudo().get_param('rust_api.base_url')
        if not existing_url:
            self.env['ir.config_parameter'].sudo().set_param(
                'rust_api.base_url', 'http://127.0.0.1:5150'
            )

    @api.depends('rust_shop_admin_id')
    def _compute_rust_shop_admin_id(self):
        for record in self:
            email = self.env['ir.config_parameter'].sudo().get_param('rust_api.shop_admin_email')
            if email:
                user = self.env['res.users'].sudo().search([('login', '=', email)], limit=1)
                record.rust_shop_admin_id = user.id if user else False
            else:
                record.rust_shop_admin_id = False

    def get_values(self):
        res = super(ResConfigSettings, self).get_values()
        email = self.env['ir.config_parameter'].sudo().get_param('rust_api.shop_admin_email')
        if email:
            user = self.env['res.users'].sudo().search([('login', '=', email)], limit=1)
            res['rust_shop_admin_id'] = user.id if user else False
        else:
            res['rust_shop_admin_id'] = False
        return res

    def set_values(self):
        super(ResConfigSettings, self).set_values()
        admin_email = False
        if self.rust_shop_admin_id:
            admin_email = self.rust_shop_admin_id.login
            self.env['ir.config_parameter'].sudo().set_param('rust_api.shop_admin_email', admin_email)
        else:
            self.env['ir.config_parameter'].sudo().set_param('rust_api.shop_admin_email', False)

        old_email = self.env['ir.config_parameter'].sudo().get_param('rust_api.shop_admin_email')
        if old_email and old_email != admin_email:
            self._send_admin_webhook(old_email, 'demote')
        if admin_email:
            self._send_admin_webhook(admin_email, 'promote')

    def _send_admin_webhook(self, email, action):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token:
            _logger.warning("No se envió webhook admin: webhook_token no configurado")
            return

        base_url = self._get_rust_base_url()
        url = f"{base_url}/api/webhooks/odoo/admin"
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }
        payload = {
            "email": email,
            "action": action,
        }

        def send_after_commit():
            try:
                response = requests.post(url, json=payload, headers=headers, timeout=5)
                if response.status_code == 200:
                    _logger.info("Admin webhook OK: %s %s", action, email)
                else:
                    _logger.warning("Admin webhook respondió %s: %s", response.status_code, response.text)
            except Exception as e:
                _logger.error("Admin webhook falló: %s", str(e))

        self.env.cr.postcommit.add(send_after_commit)

    def _get_rust_base_url(self):
        return self.env['ir.config_parameter'].sudo().get_param(
            'rust_api.base_url',
            default='http://127.0.0.1:5150'
        ).rstrip('/')
