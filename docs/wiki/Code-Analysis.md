# Code Analysis

Herramientas para analizar calidad, seguridad y dependencias del proyecto.

> Documentación completa con ejemplos: [`docs/code-analysis-tools.md`](../code-analysis-tools.md)

## Resumen rápido

```bash
# Verificación completa antes de commit
cd rust_backend
cargo fmt --all -- --check && \
cargo clippy --all-features --all && \
cargo test --all-features --all && \
cargo audit
```

## Instaladas

| Herramienta | Comando | Para qué |
|-------------|---------|----------|
| cargo-audit | `cargo audit` | Vulnerabilidades en dependencias |
| cargo-deny | `cargo deny check` | Audit + licencias + fuentes |
| cargo-clippy | `cargo clippy` | Lints de calidad y bugs (en CI) |
| cargo-fmt | `cargo fmt --check` | Formato de código (en CI) |
| cargo-test | `cargo test` | Tests unitarios + integración (en CI) |
| cargo-check | `cargo check` | Type-check rápido |
| cargo-watch | `cargo watch -x test` | Re-ejecución automática |

## Opcionales

| Herramienta | Instalar | Para qué |
|-------------|----------|----------|
| cargo-udeps | `cargo install cargo-udeps` | Dependencias no usadas |
| cargo-outdated | `cargo install cargo-outdated` | Dependencias desactualizadas |
| cargo-nextest | `cargo install cargo-nextest` | Test runner más rápido |
| ESLint | `npm install -D eslint` | Lints JavaScript/Vue |
| Prettier | `npm install -D prettier` | Formateo frontend |

## CI

GitHub Actions corre: `fmt --check` → `clippy` → `test`.
