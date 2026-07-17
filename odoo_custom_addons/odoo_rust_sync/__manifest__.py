# -*- coding: utf-8 -*-
{
    'name': "odoo_rust_sync",

    'summary': "Sincroniza productos con el backend de Rust mediante webhooks",

    'description': """
Long description of module's purpose
    """,

    'author': "My Company",
    'website': "https://www.yourcompany.com",

    # Categories can be used to filter modules in modules listing
    # Check https://github.com/odoo/odoo/blob/15.0/odoo/addons/base/data/ir_module_category_data.xml
    # for the full list
    'category': 'website',
    'version': '0.1',

    # any module necessary for this one to work correctly
    'depends': ['base','product','sale','account','payment','website'],

    # always loaded
    'data': [
        'data/ir_config_parameter_data.xml',
        'data/cron_data.xml',
        'security/ir.model.access.csv',
        'views/product_webhook_actions.xml',
        'views/shop_sync_views.xml',
        'views/shipping_views.xml',
        'views/queue_views.xml',
        #'views/templates.xml',
    ],
    # only loaded in demonstration mode
    'demo': [
        #'demo/demo.xml',
    ],
}

