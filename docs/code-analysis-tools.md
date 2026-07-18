# Code Analysis Tools

Guía de herramientas para analizar calidad, seguridad y dependencias del proyecto.

## Stack cubierto: Rust, Vue 3, Odoo (Python)

---

## Rust — herramientas instaladas

### cargo audit — vulnerabilidades en dependencias

```bash
# Escanear dependencias contra RustSec Advisory Database
cargo audit

# Solo vulnerabilidades (sin warnings de crates no mantenidos)
cargo audit --ignore RUSTSEC-2025-0057
```

Busca CVEs conocidas en dependencias directas y transitivas. La base se actualiza
periódicamente via `cargo audit` (conexión a GitHub). Si hay bloqueo de red,
usar `cargo audit --db <path-local>` con una copia local de la advisory DB.

### cargo deny — auditoría múltiple

```bash
# Verificar todo: vulnerabilidades + licencias + fuentes
cargo deny check

# Solo licencias
cargo deny check licenses

# Solo fuentes bloqueadas
cargo deny check sources

# Solo vulnerabilidades
cargo deny check advisories

# Generar plantilla de configuración
cargo deny init
```

Configuración en `rust_backend/deny.toml` (crear si no existe). Permite:
- **Advisories**: igual que `cargo audit` pero integrado.
- **Licenses**: rechazar licenses no permitidas (ej. GPL-3.0 en proyecto MIT).
- **Sources**: bloquear orígenes no autorizados (ej. repositorios git privados).

### cargo clippy — lints de Rust

```bash
# Analizar todos los targets
cargo clippy --all-features --all

# Aplicar correcciones automáticas
cargo clippy --fix --all-features

# Solo el workspace principal
cargo clippy --lib -p odoo_shop

# Ignorar warnings específicos
cargo clippy -- --allow clippy::needless_update
```

Reglas del proyecto (`.rustfmt.toml`): `max_width = 100`, `use_small_heuristics = "Default"`.

### cargo fmt — formato de código

```bash
# Verificar formato (CI)
cargo fmt --all -- --check

# Aplicar formato
cargo fmt --all
```

### cargo test — tests

```bash
# Todos los tests (unitarios + integración)
cargo test --all-features --all

# Tests específicos por nombre
cargo test checkout -- --nocapture

# Sin compilar tests (solo build)
cargo test --no-run

# Con logging
RUST_LOG=debug cargo test
```

Requiere PostgreSQL y Redis corriendo. Usa la DB configurada en `config/test.yaml`.

### cargo check — verificación rápida

```bash
# Type-check sin generar binario (más rápido que build)
cargo check

# Congelado (no toca lock, no accede a red)
cargo check --frozen
```

---

## Rust — herramientas opcionales

### cargo udeps — dependencias no usadas

```bash
# Instalar
cargo install cargo-udeps

# Detectar dependencias no utilizadas (requiere nightly)
cargo +nightly udeps
```

Analiza el árbol de dependencias y reporta crates declarados en Cargo.toml
que no se usan en el código. Útil después de refactors grandes.

### cargo outdated — dependencias desactualizadas

```bash
# Instalar
cargo install cargo-outdated

# Mostrar dependencias con versiones más nuevas
cargo outdated

# Solo dependencias directas
cargo outdated --root-deps-only
```

Compara versiones en Cargo.lock contra crates.io. Muestra cuáles están
desactualizadas y cuáles tienen breaking changes.

### cargo nextest — test runner rápido

```bash
# Instalar
cargo install cargo-nextest

# Ejecutar tests (más rápido que cargo test)
cargo nextest run

# Con perfil de CI
cargo nextest run --profile ci
```

Corre tests en paralelo con mejor reporte de fallos, timeout por test,
y rerun de tests fallidos.

### cargo watch — re-ejecución automática

```bash
# Instalado
cargo watch -x test                # re-ejecuta tests al cambiar archivos
cargo watch -x "clippy --lib"      # re-ejecuta clippy
cargo watch -x check               # re-ejecuta type-check
```

---

## Rust — comando rápido todo-en-uno

```bash
# Verificar todo antes de commit/push
cargo fmt --all -- --check && \
cargo clippy --all-features --all && \
cargo test --all-features --all && \
cargo audit
```

---

## Frontend — Vue 3 + HTMX + Tailwind CSS v4

No hay linters automáticos configurados en el proyecto. Opciones:

### ESLint — lints JavaScript/Vue

```bash
npm install -D eslint @eslint/js eslint-plugin-vue
npx eslint assets/static/js/
```

Detecta errores de sintaxis, variables no usadas, malas prácticas en Vue.

### Prettier — formateo

```bash
npm install -D prettier
npx prettier --check assets/static/js/
```

### Tailwind CSS — build

```bash
# Ya configurado. Build manual:
npx @tailwindcss/cli -i assets/static/css/tailwind-input.css \
  -o assets/static/css/tailwind.css --minify
```

---

## Odoo 18.0 (Python)

Odoo incluye su propio sistema de logging y debugging. No hay linters
de Python configurados en este proyecto. Si se trabaja en los addons
(`odoo_custom_addons/`), se recomienda:

```bash
pip install flake8 pylint
pylint odoo_custom_addons/odoo_rust_sync/
```

---

## CI actual (GitHub Actions)

Pipeline definido en `.github/workflows/ci.yaml`:

```
cargo fmt --check  →  cargo clippy  →  cargo test
```

No corre `cargo audit` ni `cargo deny` por problemas de conectividad
con crates.io en el runner. Se ejecutan manualmente en desarrollo.

---

## Referencias

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-deny documentation](https://docs.rs/cargo-deny)
- [Clippy lint list](https://rust-lang.github.io/rust-clippy/master/)
