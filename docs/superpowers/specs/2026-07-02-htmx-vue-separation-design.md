# HTMX/Vue Separation Refinement

> **For agentic workers:** Implementación directa (sin subagentes) con verificación después de cada paso.

**Goal:** Eliminar la duplicación entre HTMX y Vue, unificando todo add-to-cart bajo HTMX, y de paso migrar Tailwind a CSS build y Vue a production build.

**Architecture:** Tres cambios independientes + un fix incidental. Se pueden hacer en cualquier orden porque no se pisan.

**Tech Stack:** HTMX 1.9, Vue 3 global build, Tailwind CSS 3, Rust/Loco.rs, Tera templates

---

### Task 1: Limpiar shop.js + home.html

**Files:**
- Modify: `rust_backend/assets/static/js/shop.js` — eliminar `addToCart()`
- Modify: `rust_backend/assets/views/shop/home.html` — botones a HTMX

**Lo que pasa:** shop.js tiene un `addToCart()` que hace `fetch('/api/carts', ...)` duplicando el botón HTMX de product_detail.html. El catálogo en home.html llama a ese método Vue. La idea es que home.html use el mismo patrón HTMX que product_detail.html.

**shop.js — eliminar:**
- El método `addToCart()` entero (líneas 32-67)
- El `addingToCart` del data (línea 13)
- El import de `fetch` (la función `addToCart` lo usaba)
- shop.js queda solo con: `searchQuery`, `filteredProducts`, `handleImageError`

**home.html** — los botones de "Agregar al Carrito" cambian de:
```html
<button @click="addToCart({{ product.id }})" class="...">
  Agregar
</button>
```
a:
```html
<button hx-post="/api/carts"
        hx-vals='{"product_id": {{ product.id }} }'
        hx-swap="none"
        hx-on::after-request="document.dispatchEvent(new CustomEvent('update-cart-count')); showToast('Producto agregado al carrito')"
        class="...">
  Agregar
</button>
```

### Task 2: Vue production build

**Files:**
- Modify: `rust_backend/assets/views/base.html` — ruta de vue.global.js
- Modify: `rust_backend/assets/views/auth/login.html` — ruta de vue.global.js

**Qué cambia:**
- `vue.global.js` (582KB, dev) → `vue.global.prod.js` (~290KB, prod)
- Las rutas en los templates se actualizan
- CSP sigue necesitando `'unsafe-eval'` porque el global build compila templates en runtime

### Task 3: Tailwind CSS build

**Files:**
- Create: `rust_backend/tailwind.config.js`
- Create: `rust_backend/assets/static/css/tailwind-input.css`
- Modify: `rust_backend/assets/views/base.html` — reemplazar script tailwind por link CSS
- Modify: `rust_backend/assets/views/auth/login.html` — idem
- Modify: `rust_backend/assets/views/auth/register.html` — reemplazar CDN tailwind + htmx por local
- Delete: `rust_backend/assets/static/js/tailwind.js` (692KB)

**Pasos:**
1. `cd rust_backend && npm install -D tailwindcss`
2. Crear `tailwind.config.js`:
```js
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["assets/views/**/*.html", "assets/static/js/*.js"],
  theme: { extend: {} },
  plugins: [],
}
```
3. Crear `assets/static/css/tailwind-input.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```
4. Build: `npx tailwindcss -i assets/static/css/tailwind-input.css -o assets/static/css/tailwind.css --minify`
5. En templates: `<script src="/static/js/tailwind.js">` → `<link href="/static/css/tailwind.css" rel="stylesheet">` en **base.html**, **login.html** y **register.html**
6. En register.html: sacar CDN tailwind + CDN htmx. Poner links locales: `/static/js/htmx.min.js`, `/static/js/json-enc.js` (descargar de unpkg al dir static)
7. Eliminar `tailwind.js`

**Nota:** register.html y login.html son standalone (no extienden base.html). Cada uno necesita sus propios links a tailwind.css, htmx.min.js y (register) json-enc.js.

### Task 4: Fix login.html typo

**Files:**
- Modify: `rust_backend/assets/views/auth/login.html`

**Qué cambia:**
```html
<script src="/static/js/vue.global.js"></script>>
```
→
```html
<script src="/static/js/vue.global.prod.js"></script>
```

### Orden de implementación

3 → 2 → 1 → 4 (Tailwind primero porque es el cambio más pesado, shop.js al final porque es la cirugía fina)

### Verification
- `cargo check --all-features` (solo toca assets, pero por las dudas)
- Visual: cargar home, product_detail, cart, checkout, login, register — verificar que no hay errores en consola
- Visual: agregar producto al carrito desde home y desde product_detail — verificar badge se actualiza
