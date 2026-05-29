# Tienda Total Odoo 🏪📦

[![Odoo](https://img.shields.io/badge/Odoo-18.0-875A7B.svg?logo=odoo&logoColor=white)](https://www.odoo.com)
[![Redis](https://img.shields.io/badge/Redis-Asíncrono-DC382D.svg?logo=redis&logoColor=white)](https://redis.io)
[![License: LGPL-3](https://img.shields.io/badge/License-LGPL--3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.html)

**Tienda Total Odoo** es una suite de módulos y personalizaciones diseñada para optimizar, centralizar y potenciar la gestión integral de tiendas minoristas (Retail), supermercados y comercios medianos. 

A diferencia de las implementaciones estándar, este proyecto desacopla y potencia el sitio web mediante la integración del ecosistema **Loco** y **Redis**, garantizando una velocidad de respuesta masiva y un entorno de alta seguridad para operaciones críticas.

---

## 📋 Tabla de Contenidos
- [Características Principales](#-características-principales)
- [Arquitectura Avanzada: Loco + Redis](#-arquitectura-avanzada-loco--redis)
- [Requisitos del Sistema](#️-requisitos-del-sistema)
- [Instalación y Configuración](#️-instalación-y-configuración)
- [Uso](#-uso)
- [Módulos Incluidos](#-módulos-incluidos)
- [Solución de Problemas](#-solución-de-problemas)
- [Contribuir y Soporte](#-contribuir)
- [Licencia](#-licencia)

---

## 🚀 Características Principales

* **Punto de Venta (POS) Enriquecido:** Soporte multi-caja, integración de periféricos (básculas, lectores) y control estricto de arqueos.
* **Gestión de Inventario Avanzada:** Control de stock mínimo, alertas de reposición, trazabilidad por lotes/caducidad y transferencias multi-sucursal.
* **Facturación Electrónica:** Sincronización directa del flujo de caja del POS con los diarios contables y pasarelas fiscales.
* **Fidelización:** Monedero electrónico, sistemas de puntos, tarjetas de regalo y promociones dinámicas complejos.

---

## ⚡ Arquitectura Avanzada: Loco + Redis

Para superar las limitaciones de rendimiento y concurrencia de las herramientas nativas de Odoo en el sitio web, este proyecto implementa una infraestructura basada en **Loco**:

### 🛠️ Optimización de Procesos y Consultas (Redis)
* **Caché de Consultas:** Se utiliza Redis como capa de almacenamiento en caché en memoria para las consultas más frecuentes de la base de datos de Odoo, reduciendo drásticamente los tiempos de carga del catálogo y la saturación de PostgreSQL.
* **Encolamiento Asíncrono de Tareas:** Los procesos pesados del sitio web (procesamiento de pagos, generación de facturas, sincronización de stock masivo y envíos de correo) se derivan a colas de trabajo gestionadas por Redis, evitando el bloqueo del hilo principal de Odoo.

### 🔒 Blindaje y Seguridad del Servidor
A través de las **librerías dedicadas de Loco**, el proyecto sustituye los controladores web nativos para elevar los estándares de protección en dos frentes:
* **Integridad del Servidor:** Mitigación activa contra ataques de denegación de servicio (DoS), inyecciones de código y accesos no autorizados a la API del backend.
* **Navegación Segura del Usuario:** Implementación estricta de políticas de seguridad (CSP), prevención de falsificación de peticiones en sitios cruzados (CSRF) y desinfección de entradas para neutralizar ataques XSS, manteniendo a los clientes fuera de peligro durante todo el flujo de compra.

---

## 🛠️ Requisitos del Sistema

* **Odoo Versión:** 18.0 (Community o Enterprise)
* **Base de datos:** PostgreSQL 13 o superior
* **Servidor de Memoria:** Redis Server 6.x o superior
* **Librerías Python adicionales:**
    * `redis` (Para la persistencia y comunicación con el backend de caché)
    * `pandas` (Para exportación de reportes)
    * Librerías nativas y de seguridad del ecosistema **Loco**.

---

## ⚙️ Instalación y Configuración

Siga estos pasos para integrar el repositorio en su instancia de Odoo:

### 1. Clonar el repositorio
Acceda al directorio de addons personalizados y clone el proyecto:
```bash
cd /opt/odoo/custom_addons
git clone [https://github.com/andybatc/tienda_total_odoo.git](https://github.com/andybatc/tienda_total_odoo.git)
