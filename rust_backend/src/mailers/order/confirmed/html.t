<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; color: #333; }
        .container { max-width: 600px; margin: 0 auto; padding: 20px; }
        .header { background: #1e293b; color: white; padding: 20px; text-align: center; border-radius: 8px 8px 0 0; }
        .body { padding: 20px; border: 1px solid #e2e8f0; border-top: none; border-radius: 0 0 8px 8px; }
        .order-info { background: #f8fafc; padding: 15px; border-radius: 6px; margin: 15px 0; }
        .order-info p { margin: 5px 0; }
        .footer { text-align: center; color: #94a3b8; font-size: 12px; margin-top: 20px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>¡Gracias por tu compra!</h1>
        </div>
        <div class="body">
            <p>Hola <strong>{{ customer_name }}</strong>,</p>
            <p>Tu orden ha sido confirmada exitosamente.</p>
            <div class="order-info">
                <p><strong>Orden:</strong> {{ order_name }}</p>
                <p><strong>Total:</strong> ${{ total }}</p>
                <p><strong>Estado:</strong> {{ status }}</p>
            </div>
            <p>Recibirás actualizaciones sobre el envío en este correo.</p>
            <p>Saludos,<br>OdooShop</p>
        </div>
        <div class="footer">
            <p>OdooShop &mdash; Powered by Loco.rs + Odoo</p>
        </div>
    </div>
</body>
</html>
