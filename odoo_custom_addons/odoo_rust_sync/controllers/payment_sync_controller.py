import json
import logging
import requests

from odoo import http
from odoo.http import Response, request

_logger = logging.getLogger(__name__)


class PaymentSyncController(http.Controller):

    @http.route('/api/config/payment-methods', type='http', auth='public', methods=['GET'], csrf=False)
    def get_payment_methods(self, **kwargs):
        token = request.httprequest.headers.get('Authorization', '').replace('Bearer ', '')
        expected_token = request.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token or token != expected_token:
            return Response(json.dumps({'error': 'Unauthorized'}), status=401, content_type='application/json')

        providers = request.env['payment.provider'].sudo().search([('state', '!=', 'disabled')])
        result = [{
            'odoo_provider_id': p.id,
            'name': p.name,
            'code': p.code,
            'state': p.state,
            'is_published': p.is_published,
            'allow_tokenization': p.allow_tokenization,
            'capture_manually': p.capture_manually,
            'sequence': p.sequence,
        } for p in providers]

        return Response(json.dumps(result), content_type='application/json')

    @http.route('/api/config/payment-methods/sync', type='http', auth='public', methods=['POST'], csrf=False)
    def sync_payment_methods(self, **kwargs):
        token = request.httprequest.headers.get('Authorization', '').replace('Bearer ', '')
        expected_token = request.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token or token != expected_token:
            return Response(json.dumps({'error': 'Unauthorized'}), status=401, content_type='application/json')

        providers = request.env['payment.provider'].sudo().search([('state', '!=', 'disabled')])
        batch = [{
            'odoo_provider_id': p.id,
            'name': p.name,
            'code': p.code,
            'state': p.state,
            'is_published': p.is_published,
            'allow_tokenization': p.allow_tokenization,
            'capture_manually': p.capture_manually,
            'sequence': p.sequence,
        } for p in providers]

        base_url = request.env['ir.config_parameter'].sudo().get_param('rust_api.base_url', 'http://127.0.0.1:5150').rstrip('/')
        webhook_url = f"{base_url}/api/webhooks/odoo/payment-methods"
        headers = {
            "Authorization": f"Bearer {expected_token}",
            "Content-Type": "application/json"
        }

        try:
            ext_resp = requests.post(webhook_url, json=batch, headers=headers, timeout=10)
            if ext_resp.status_code == 200:
                return Response(json.dumps({'status': 'ok', 'synced': len(batch)}), content_type='application/json')
            else:
                return Response(json.dumps({'error': 'Rust returned %d' % ext_resp.status_code}), status=502, content_type='application/json')
        except Exception as e:
            return Response(json.dumps({'error': str(e)}), status=502, content_type='application/json')
