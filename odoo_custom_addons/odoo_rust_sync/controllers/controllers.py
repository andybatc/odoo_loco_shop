import json
import logging

from odoo import http
from odoo.exceptions import AccessError, UserError, ValidationError
from odoo.http import Response, request

_logger = logging.getLogger(__name__)


class OdooOrderController(http.Controller):

    @http.route('/api/orders/create', type='http', auth='public', methods=['POST'], csrf=False)
    def create_order(self, **kwargs):
        token = request.httprequest.headers.get('Authorization', '').replace('Bearer ', '')
        expected_token = request.env['ir.config_parameter'].sudo().get_param('rust_api.webhook_token')
        if not token or token != expected_token:
            return self._json_response({'error': 'Unauthorized'}, 401)

        try:
            data = json.loads(request.httprequest.data)
        except ValueError:
            return self._json_response({'error': 'Invalid JSON'}, 400)

        customer = data.get('customer', {})
        payment_method_id = data.get('payment_method_id')
        items = data.get('items', [])

        if not items:
            return self._json_response({'error': 'No items in order'}, 400)

        partner = None
        partner_email = customer.get('email', '').strip()
        if partner_email:
            partner = request.env['res.partner'].sudo().search([('email', '=', partner_email)], limit=1)
        if not partner:
            partner = request.env['res.partner'].sudo().create({
                'name': customer.get('name', 'Guest').strip() or 'Guest',
                'email': partner_email or False,
                'phone': customer.get('phone', '').strip() or False,
                'street': customer.get('street', '').strip() or False,
                'city': customer.get('city', '').strip() or False,
                'zip': customer.get('zip', '').strip() or False,
                'customer_rank': 1,
            })
        elif partner_email and partner.email != partner_email:
            partner.sudo().write({'email': partner_email})

        order_lines = []
        for item in items:
            product_tmpl_id = item.get('product_id')
            if not product_tmpl_id:
                continue
            product = request.env['product.product'].sudo().search(
                [('product_tmpl_id', '=', product_tmpl_id)], limit=1
            )
            if not product:
                _logger.warning("Product not found for template ID: %s", product_tmpl_id)
                continue
            order_lines.append((0, 0, {
                'product_id': product.id,
                'name': item.get('name', 'Product'),
                'product_uom_qty': item.get('quantity', 1),
                'price_unit': item.get('price', 0.0),
            }))

        if not order_lines:
            return self._json_response({'error': 'No valid products found in Odoo'}, 400)

        try:
            sale_vals = {
                'partner_id': partner.id,
                'order_line': order_lines,
            }
            if payment_method_id:
                provider = request.env['payment.provider'].sudo().browse(payment_method_id)
                if provider.exists():
                    sale_vals['note'] = "Payment Method: %s (%s)" % (provider.name, provider.code)
            sale_order = request.env['sale.order'].sudo().create(sale_vals)

            sale_order.action_confirm()

            for line in sale_order.order_line:
                line.qty_delivered = line.product_uom_qty

            invoices = sale_order._create_invoices()
            if invoices:
                invoices.action_post()
                invoice = invoices[0]
                invoice_name = invoice.name
                invoice_id = invoice.id
            else:
                invoice_name = False
                invoice_id = False

            return self._json_response({
                'order_name': sale_order.name,
                'order_id': sale_order.id,
                'invoice_name': invoice_name,
                'invoice_id': invoice_id,
            })
        except (UserError, AccessError, ValidationError) as e:
            _logger.error("Error creating order: %s", e, exc_info=True)
            return self._json_response({'error': str(e)}, 500)

    def _json_response(self, data, status=200):
        return Response(
            json.dumps(data),
            status=status,
            content_type='application/json',
        )
