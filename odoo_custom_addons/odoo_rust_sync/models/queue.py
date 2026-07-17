from odoo import models, fields, api
import json
import logging
import requests
from datetime import datetime, timedelta

_logger = logging.getLogger(__name__)


class RustWebhookQueue(models.Model):
    _name = 'rust_webhook.queue'
    _description = 'Webhook Queue for Rust Backend'
    _order = 'next_retry_at ASC, created_at ASC'

    model_name = fields.Char(string='Odoo Model', required=True)
    res_id = fields.Integer(string='Record ID', required=True)
    webhook_type = fields.Selection([
        ('product_update', 'Product Update'),
        ('payment_sync', 'Payment Sync'),
        ('shipping_sync', 'Shipping Rate Sync'),
    ], string='Webhook Type', required=True)
    payload = fields.Text(string='Payload (JSON)', required=True)
    state = fields.Selection([
        ('pending', 'Pending'),
        ('processing', 'Processing'),
        ('done', 'Done'),
        ('failed', 'Failed'),
    ], string='State', default='pending', required=True)
    retry_count = fields.Integer(string='Retry Count', default=0)
    max_retries = fields.Integer(string='Max Retries', default=5)
    last_error = fields.Text(string='Last Error')
    next_retry_at = fields.Datetime(string='Next Retry At')
    created_at = fields.Datetime(string='Created At', default=fields.Datetime.now)

    def _get_backoff_seconds(self):
        """Exponential backoff: 1min, 5min, 15min, 1h, 6h"""
        backoffs = [60, 300, 900, 3600, 21600]
        idx = min(self.retry_count, len(backoffs) - 1)
        return backoffs[idx]

    def _get_webhook_url(self):
        base_url = self.env['ir.config_parameter'].sudo().get_param(
            'rust_api.base_url', default='http://127.0.0.1:5150').rstrip('/')
        urls = {
            'product_update': '/api/webhooks/odoo/update',
            'payment_sync': '/api/webhooks/odoo/payment-methods',
            'shipping_sync': '/api/shipping/rates/sync',
        }
        return base_url + urls.get(self.webhook_type, '')

    def _get_headers(self):
        token = self.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        return {
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json',
        }

    def _send(self):
        """Actual HTTP call to Rust backend"""
        self.ensure_one()
        if self.state == 'done':
            return True

        self.write({'state': 'processing'})

        url = self._get_webhook_url()
        headers = self._get_headers()

        try:
            payload_data = json.loads(self.payload)
            if self.webhook_type in ('product_update',):
                # Product update needs single object, not array
                resp = requests.post(url, json=payload_data, headers=headers, timeout=5)
            elif self.webhook_type == 'payment_sync':
                # Payment sync needs array
                data = payload_data if isinstance(payload_data, list) else [payload_data]
                resp = requests.post(url, json=data, headers=headers, timeout=5)
            elif self.webhook_type == 'shipping_sync':
                # Shipping sync has its own format: {"rates": [...]}
                resp = requests.post(url, json=payload_data, headers=headers, timeout=5)
            else:
                resp = requests.post(url, json=payload_data, headers=headers, timeout=5)

            if resp.status_code in (200, 202):
                self.write({'state': 'done', 'last_error': False})
                _logger.info(f"Webhook {self.webhook_type} for {self.model_name}:{self.res_id} sent successfully")
                return True
            else:
                raise Exception(f"HTTP {resp.status_code}: {resp.text[:200]}")
        except Exception as e:
            retry = self.retry_count + 1
            backoff = self._get_backoff_seconds()
            next_retry = datetime.now() + timedelta(seconds=backoff)

            if retry >= self.max_retries:
                self.write({
                    'state': 'failed',
                    'retry_count': retry,
                    'last_error': str(e),
                    'next_retry_at': False,  # no more retries
                })
                _logger.error(f"Webhook {self.webhook_type} for {self.model_name}:{self.res_id} FAILED after {retry} retries: {e}")
            else:
                self.write({
                    'state': 'failed',
                    'retry_count': retry,
                    'last_error': str(e),
                    'next_retry_at': next_retry,
                })
                _logger.warning(f"Webhook {self.webhook_type} for {self.model_name}:{self.res_id} failed (retry {retry}/{self.max_retries}), next at {next_retry}: {e}")
            return False

    @api.model
    def cron_retry_webhooks(self):
        """Cron job: process pending and failed webhooks that are due for retry"""
        now = fields.Datetime.now()
        records = self.search([
            ('state', 'in', ('pending', 'failed')),
            '|',
            ('next_retry_at', '<=', now),
            ('next_retry_at', '=', False),  # pending without next_retry_at set
        ], limit=50)

        for record in records:
            try:
                if not record.next_retry_at:
                    # First try for pending records
                    record._send()
                elif record.next_retry_at <= now:
                    record._send()
            except Exception as e:
                _logger.error(f"Error in cron processing webhook {record.id}: {e}")

        return True

    @api.model
    def enqueue(self, model_name, res_id, webhook_type, payload):
        """Create a queue record and attempt immediate send"""
        record = self.create({
            'model_name': model_name,
            'res_id': res_id,
            'webhook_type': webhook_type,
            'payload': json.dumps(payload) if not isinstance(payload, str) else payload,
            'state': 'pending',
        })

        # Try immediately
        try:
            record._send()
        except Exception:
            pass  # cron will retry

        return record
