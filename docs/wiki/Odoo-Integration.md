# Odoo Integration

Odoo actúa como ERP del catálogo web. La comunicación es bidireccional:

- **Odoo → Rust (webhooks)**: productos, métodos de pago
- **Rust → Odoo (HTTP)**: creación de pedidos y facturas

## Addons

### odoo_rust_sync

Addon principal de integración. Ubicado en `odoo_custom_addons/odoo_rust_sync/`.

**Modelos**:
| Archivo | Función |
|---------|---------|
| `models/product_template_inherit.py` | Webhooks al crear/modificar productos |
| `models/payment_provider_inherit.py` | Webhooks al crear/modificar métodos de pago |
| `models/res_config_settings_inherit.py` | Configuración del token y base_url |
| `models/website_inherit.py` | Override de `_is_canonical_url` |

**Controladores** (endpoints HTTP, autenticados con Bearer token):
| Ruta | Método | Función |
|------|--------|---------|
| `/api/orders/create` | POST | Crear partner + orden de venta + factura |
| `/api/config/payment-methods` | GET | Listar métodos de pago activos |
| `/api/config/payment-methods/sync` | POST | Sincronizar métodos de pago al backend |

### muk_web_*

Conjunto de addons de MuK para personalizar la interfaz backend de Odoo:

| Addon | Función |
|-------|---------|
| `muk_web_theme` | Tema visual completo (navbar, colores, favicon) |
| `muk_web_appsbar` | Barra lateral de aplicaciones con logo |
| `muk_web_dialog` | Diálogos a pantalla completa |
| `muk_web_chatter` | Mejoras en el chatter |
| `muk_web_colors` | Personalización de colores por compañía |

## Despliegue en Docker

### Cómo se montan los addons

Cuando levantás Odoo con `docker compose --profile full up`, pasan estas cosas:

**1. Build de la imagen custom**

`odoo_custom_addons/Dockerfile`:
```dockerfile
FROM odoo:18
USER root
RUN pip3 install --no-cache-dir requests
USER odoo
```

La imagen oficial `odoo:18` no incluye la librería `requests`. Pero `odoo_rust_sync` la importa a nivel de módulo en 4 archivos. Sin esto, Odoo crashea al arrancar.

**2. Montaje de addons como volumen**

```yaml
volumes:
  - ./odoo_custom_addons:/mnt/extra-addons
```

La imagen oficial de Odoo configura automáticamente el path `/mnt/extra-addons` en su `--addons-path`. Cualquier addon colocado ahí es detectable por Odoo sin configuración adicional.

**3. Auto-inicialización de la base de datos**

```yaml
command: odoo -d odoo_prod -i muk_web_theme,odoo_rust_sync --without-demo=all
```

- `-d odoo_prod`: nombre de la base de datos
- `-i muk_web_theme,odoo_rust_sync`: módulos a instalar (el resto se instalan por dependencia transitiva)
- `--without-demo=all`: evita cargar datos demo

En el primer arranque, Odoo detecta que la BD no existe, la crea e instala los módulos. En arranques siguientes, la BD existe y el `-i` se ignora.

**4. Conexión a PostgreSQL externa**

```yaml
environment:
  HOST: postgres        # servicio Docker
  USER: postgres
  PASSWORD: postgres
```

Odoo se conecta al servicio `postgres` del compose, no a una BD local.

## Configuración

### Webhook Token

Cada request de Odoo a Rust incluye el header `Authorization: Bearer <token>`.

**El token se configura en dos lugares** (deben coincidir):

1. **Odoo**: `rust_api.webhook_token` en Ajustes → Técnico → Parámetros del sistema
2. **Rust**: `webhook_token` en la tabla `configs` (gestionable via UI en `http://localhost:5150/ui/auth/token`)

El addon genera automáticamente un token al abrir la configuración del módulo en Odoo.

### Base URL

Parámetro `rust_api.base_url` en Odoo → Parámetros del sistema.

- Con Docker: `http://rust_backend:5150`
- Sin Docker: `http://localhost:5150` (o `http://host.docker.internal:5150`)
- En producción: URL pública del backend

## Verificar integración

### Webhooks de producto

1. En Odoo, crear nuevo producto (Ventas → Productos → Crear)
2. Completar nombre, precio, guardar
3. En logs del backend Rust: debe aparecer `"Webhook received for product X"`
4. Navegar a `http://localhost:5150/shop/home` — el producto aparece

### Creación de pedido

1. En el shop, agregar producto al carrito
2. Ir a checkout, completar datos, confirmar
3. En Odoo, buscar la orden de venta (Ventas → Órdenes)
4. La orden debe estar confirmada con factura validada

## Solución de problemas

| Síntoma | Causa probable |
|---------|---------------|
| 401 en webhooks | Token no coincide entre Odoo y Rust |
| Webhook no llega | `base_url` incorrecto o firewall |
| Producto no aparece en shop | Cache Redis no invalidada (esperar TTL o reiniciar) |
| Error 500 en checkout | Odoo no alcanzable desde el backend Rust |
| 403 en Docker | CORS: el origin no está en la whitelist |
