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
use axum::routing::get_service;
use loco_rs::prelude::*;
use tower_http::services::ServeDir;
use loco_rs::controller::Routes;
use std::env;

#[allow(unused_imports)]
use crate::{controllers, models::_entities::users, tasks, workers::downloader::DownloadWorker};

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
            .add_route(controllers::homepage::routes())
            .add_route(controllers::token_auth::routes())
            .add_route(controllers::views::routes())
            .add_route(controllers::config::routes())
            .add_route(controllers::products_webhook::routes())
            .add_route(controllers::shop::routes())
            .add_route(controllers::auth::routes())
    }
    async fn after_routes(router: Router, _ctx: &AppContext) -> Result<Router> {
        // 1. Construimos la ruta absoluta hacia tus imágenes
        let root = std::env::current_dir().expect("No se pudo obtener el directorio actual");
        let storage_path = root.join("storage/products");

        // 2. Usamos nest_service directamente en el router final
        let router = router.nest_service("/storage/products", ServeDir::new(storage_path));

        // 3. Construimos el motor Tera
        let tera_engine = TeraView::build()?;
        
        Ok(router.layer(Extension(ViewEngine::new(tera_engine))))
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(crate::workers::webhook::WebhookWorker::build(ctx)).await?;
        queue.register(crate::workers::product_sync::Worker::build(ctx)).await?;
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::sync::Sync);
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