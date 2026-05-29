# Tienda Total Odoo 🏪📦

[![Odoo](https://img.shields.io/badge/Odoo-18.0%20%7C%2017.0-875A7B.svg?logo=odoo&logoColor=white)](https://www.odoo.com)
[![License: LGPL-3](https://img.shields.io/badge/License-LGPL--3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.html)
[![Maintained: Yes](https://img.shields.io/badge/Maintained%3F-yes-green.svg)](https://github.com/andybatc/tienda_total_odoo)

**Tienda Total Odoo** es una suite de módulos y personalizaciones diseñada para optimizar, centralizar y potenciar la gestión integral de tiendas minoristas (Retail), supermercados y comercios medianos dentro del ecosistema Odoo. Este repositorio agrupa las aplicaciones necesarias para la administración de inventario multi-sucursal, punto de venta (POS) avanzado, facturación electrónica y reportes financieros unificados.

---

## 🚀 Características Principales

* **Punto de Venta (POS) Enriquecido:**
    * Soporte para múltiples cajas e impresoras de tickets en red.
    * Integración de básculas y lectores de códigos de barras optimizados.
    * Búsqueda avanzada de productos y control de arqueos de caja por cajero.
* **Gestión de Inventario y Abastecimiento:**
    * Control estricto de stock mínimo y alertas automáticas de reposición.
    * Trazabilidad total mediante lotes, números de serie y fechas de caducidad.
    * Transferencias internas simplificadas entre sucursales/almacenes.
* **Facturación y Contabilidad Automatizada:**
    * Emisión de facturas electrónicas e integración con pasarelas fiscales locales.
    * Sincronización directa del flujo de caja del Punto de Venta con los diarios contables.
* **Fidelización de Clientes:**
    * Módulo integrado de monedero electrónico, puntos acumulables y tarjetas de regalo.
    * Gestión de promociones complejas (ej. 2x1, descuentos escalonados por volumen).
* **Reportes y Dashboards Avanzados:**
    * Métricas de rendimiento en tiempo real: productos más vendidos, márgenes de ganancia y horas pico de venta.

---

## 🛠️ Requisitos del Sistema

Para asegurar el correcto funcionamiento de los módulos de este repositorio, su entorno debe cumplir con:

* **Odoo Versión:** 18.0 o superior (Community o Enterprise)
* **Base de datos:** PostgreSQL 13 o superior
* **Librerías Python adicionales:**
    * `pandas` (para la exportación de reportes avanzados)
    * `python-barcode` / `qrcode` (para la generación automática de etiquetas)
    * `requests` (para las conexiones de facturación electrónica)

---

## ⚙️ Instalación y Configuración

Siga estos pasos para clonar e integrar este repositorio en su instancia de Odoo:

### 1. Clonar el repositorio
Acceda al directorio de addons personalizados de su servidor Odoo y clone el proyecto:

```bash
cd /opt/odoo/custom_addons
git clone [https://github.com/andybatc/tienda_total_odoo.git](https://github.com/andybatc/tienda_total_odoo.git)
