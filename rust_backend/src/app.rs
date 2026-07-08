use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;
use loco_rs::controller::views::{engines::TeraView, ViewEngine};
use axum::{Extension, Router};
use axum::http::HeaderValue;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use loco_rs::prelude::*;
use utoipa::OpenApi;
use std::env;

use crate::{controllers, models::_entities::users, middleware, tasks, workers};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::carts::routes())
            .add_route(controllers::checkout::routes())
            .add_route(controllers::homepage::routes())

            .add_route(controllers::views::routes())
            .add_route(controllers::config::routes())
            .add_route(controllers::products_webhook::routes())
            .add_route(controllers::payment_webhooks::routes())
            .add_route(controllers::shop::routes())
            .add_route(controllers::auth::routes())
            .add_route(controllers::admin::routes())
            .add_route(controllers::shipping::routes())
    }
    async fn after_routes(router: Router, ctx: &AppContext) -> Result<Router> {
        let router = router;

        let router = if ctx.environment != Environment::Test {
            let (prometheus_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
            router
                .route("/metrics", axum::routing::get(|| async move { metric_handle.render() }))
                .layer(prometheus_layer)
        } else {
            router
        };
        // Middleware para reemplazar páginas de error 401/403/500
        async fn error_page_middleware(
            req: axum::http::Request<axum::body::Body>,
            next: axum::middleware::Next,
        ) -> axum::response::Response {
            let response = next.run(req).await;
            let status = response.status();

            if status == axum::http::StatusCode::INTERNAL_SERVER_ERROR {
                if let Ok(body) = std::fs::read_to_string("assets/static/500.html") {
                    return axum::response::Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .header("content-type", "text/html")
                        .body(axum::body::Body::from(body))
                        .unwrap();
                }
            }

            if (status == axum::http::StatusCode::UNAUTHORIZED
                || status == axum::http::StatusCode::FORBIDDEN)
                && response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map_or(true, |ct| !ct.starts_with("application/json"))
            {
                if let Ok(body) = std::fs::read_to_string("assets/static/403.html") {
                    return axum::response::Response::builder()
                        .status(status)
                        .header("content-type", "text/html")
                        .body(axum::body::Body::from(body))
                        .unwrap();
                }
            }

            response
        }

        // 1. Construimos la ruta absoluta hacia tus imágenes
        let root = std::env::current_dir().expect("No se pudo obtener el directorio actual");
        let storage_path = root.join("storage/products");

        // 2. Usamos nest_service directamente en el router final
        let router = router.nest_service("/storage/products", ServeDir::new(storage_path));

        // 3. Construimos el motor Tera
        let tera_engine = TeraView::build()?;

        // 4. CORS - restrictivo, solo origenes conocidos
        let cors = CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::predicate(
                |origin: &HeaderValue, _| {
                    let origin_str = origin.to_str().unwrap_or("");
                    origin_str == "http://localhost:5150"
                        || origin_str == "http://localhost:5173"
                        || origin_str.starts_with("http://127.0.0.1")
                },
            ))
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::OPTIONS])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION, axum::http::header::HeaderName::from_static("x-csrf-token")])
            .allow_credentials(true);

        // 5. OpenAPI / Swagger UI
        use utoipa_swagger_ui::SwaggerUi;
        let router = router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", crate::api_docs::ApiDoc::openapi()));

        Ok(router
            .layer(axum::middleware::from_fn(error_page_middleware))
            .layer(axum::middleware::from_fn(middleware::security_headers::add_security_headers))
            .layer(RequestBodyLimitLayer::new(1024 * 1024))
            .layer(cors)
            .layer(Extension(ViewEngine::new(tera_engine))))
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(crate::workers::webhook::WebhookWorker::build(ctx)).await?;
        queue.register(crate::workers::product_sync::Worker::build(ctx)).await?;
        queue.register(crate::workers::order_creation::OrderCreationWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::sync::Sync);
        tasks.register(tasks::promote_user::PromoteUser);
        tasks.register(tasks::cleanup_carts::CleanupCarts);
        tasks.register(workers::shipping_sync::ShippingSyncWorker);
        // tasks-inject (do not remove)
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}