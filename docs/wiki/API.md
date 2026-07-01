# API Reference

Documentación interactiva disponible en [`/swagger-ui`](http://localhost:5150/swagger-ui) cuando el servidor está corriendo.

## Webhooks (Odoo → Rust)

Requieren header `Authorization: Bearer <token>`.

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/webhooks/odoo/update` | Producto individual |
| POST | `/api/webhooks/odoo/bulk-update` | Productos en lote |
| POST | `/api/webhooks/odoo/payment-methods` | Métodos de pago |

## Carrito

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/carts/` | Agregar item al carrito |
| GET | `/api/carts/` | Obtener carrito actual |
| DELETE | `/api/carts/{item_id}` | Remover item |
| DELETE | `/api/carts/` | Vaciar carrito |

## Checkout

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/checkout` | Procesar pedido |
| GET | `/api/checkout/payment-methods` | Métodos de pago disponibles |

## Autenticación

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/api/auth/register` | Registro de usuario |
| POST | `/api/auth/login` | Inicio de sesión |
| POST | `/api/auth/magic-link` | Magic link |
| GET | `/api/auth/current` | Usuario actual |

## Configuración

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/api/config/token` | Obtener webhook token |
| POST | `/api/config/token` | Actualizar webhook token |

## Ejemplos

```bash
# Sincronizar producto desde Odoo
curl -X POST http://localhost:5150/api/webhooks/odoo/update \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"id": 123, "name": "Producto", "list_price": 29.99, "default_code": "REF-001"}'

# Agregar al carrito
curl -X POST http://localhost:5150/api/carts/ -d '{"product_id": 123}'

# Registrar usuario
curl -X POST http://localhost:5150/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"name": "Usuario", "email": "user@example.com", "password": "secret"}'

# Checkout
curl -X POST http://localhost:5150/api/checkout \
  -H "Content-Type: application/json" \
  -d '{"customer": {"name": "Juan", "email": "juan@example.com", "phone": "555-0100", "street": "Calle 123", "city": "CIDMX"}, "cart_id": "<uuid>"}'
```
