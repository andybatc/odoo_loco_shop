from odoo import models, api
import secrets

class ResConfigSettings(models.TransientModel):
    _inherit = 'res.config.settings'

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