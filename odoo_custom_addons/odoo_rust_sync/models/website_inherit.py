from odoo import models


class Website(models.Model):
    _inherit = 'website'

    def _is_canonical_url(self):
        try:
            return super()._is_canonical_url()
        except KeyError:
            return False
