use crate::models::_entities::products;
use loco_rs::prelude::*;
use sea_orm::{Set,ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};

pub struct WebhookWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize, ToSchema)]
pub struct WebhookWorkerArgs {
    pub odoo_id: i32,
    pub name: Option<String>,
    pub price: Option<Decimal>,
    pub image_base64: Option<String>,
    pub is_published: bool,
    #[serde(default)]
    pub tax_percent: Option<f64>,
}

#[async_trait]
impl BackgroundWorker<WebhookWorkerArgs> for WebhookWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "Webhook".to_string()
    }

    async fn perform(&self, args: WebhookWorkerArgs) -> Result<()> {
        tokio::time::sleep(Duration::from_millis(500)).await;
        tracing::info!("📦 Procesando webhook local para Odoo ID: {}", args.odoo_id);

        // =========================================================
        // DECODIFICAR Y GUARDAR IMAGEN EN DISCO
        // =========================================================
        let mut guardado_image_filename: Option<String> = None;
        if let Some(ref b64_str) = args.image_base64 {
            if !b64_str.is_empty() {
                if let Ok(image_bytes) = general_purpose::STANDARD.decode(b64_str) {
                    let storage_dir = std::path::Path::new("storage/products");
                    if !storage_dir.exists() {
                        let _ = tokio::fs::create_dir_all(storage_dir).await;
                    }
                    let file_name = format!("{}.webp", uuid::Uuid::new_v4());
                    let file_path = storage_dir.join(&file_name);
                    if let Ok(_) = tokio::fs::write(file_path, image_bytes).await {
                        tracing::info!("💾 Imagen guardada localmente desde Webhook: {}", file_name);
                        guardado_image_filename = Some(file_name);
                    }
                }
            }
        }
        // =========================================================

        // Buscar en la base de datos local de la tienda por 'odoo_id'
        let local_product = products::Entity::find()
            .filter(products::Column::OdooId.eq(Some(args.odoo_id)))
            .one(&self.ctx.db)
            .await
            .map_err(|e| loco_rs::Error::msg(e))?;

        match local_product {
            // CASO A: El producto existe -> Actualizar campos de forma defensiva
            Some(existing_product) => {
                tracing::info!("🔄 Producto encontrado en tienda. Evaluando cambios...");

                let mut active_model: products::ActiveModel = existing_product.clone().into();
                let mut hubo_cambios = false;

                if let Some(name) = args.name {
                    if !name.is_empty() && name != "Sin nombre" {
                        active_model.name = Set(Some(name));
                        hubo_cambios = true;
                    }
                }

                if let Some(price) = args.price {
                    active_model.price = Set(Some(price));
                    hubo_cambios = true;
                }

                if let Some(img_file) = guardado_image_filename {
                    active_model.image_filename = Set(Some(img_file));
                    hubo_cambios = true;
                }

                if existing_product.is_published != args.is_published {
                    active_model.is_published = Set(args.is_published);
                    hubo_cambios = true;
                }

                if let Some(tax) = args.tax_percent {
                    let decimal_tax = Decimal::new((tax * 100.0).round() as i64, 2).normalize();
                    if existing_product.tax_percent != Some(decimal_tax) {
                        active_model.tax_percent = Set(Some(decimal_tax));
                        hubo_cambios = true;
                    }
                }

                if hubo_cambios {
                    active_model.updated_at = Set(chrono::Utc::now().into());
                    active_model.update(&self.ctx.db)
                        .await
                        .map_err(|e| loco_rs::Error::msg(e))?;
                    tracing::info!("💾 ¡Cambios guardados en la Base de Datos!");

                    bump_search_version(&self.ctx).await;
                    bump_catalog_version(&self.ctx).await;
                    tracing::info!("♻️ Caché de Redis invalidada para producto {} y catálogo global.", args.odoo_id);
                } else {
                    tracing::warn!("⚠️ No se detectaron cambios reales. Se omitió el UPDATE.");
                }
            }
            // CASO B: El producto no existe -> Crear uno nuevo desde cero
            None => {
                tracing::info!("✨ Producto nuevo. Insertando en la tienda para ID Odoo: {}", args.odoo_id);

                let new_product = products::ActiveModel {
                    odoo_id: Set(Some(args.odoo_id)),
                    name: Set(args.name.or(Some("Producto sin nombre".to_string()))),
                    price: Set(args.price),
                    image_filename: Set(guardado_image_filename),
                    is_published: Set(args.is_published),
                    created_at: Set(chrono::Utc::now().into()),
                    updated_at: Set(chrono::Utc::now().into()),
                    tax_percent: Set(args.tax_percent.map(|v| Decimal::new((v * 100.0).round() as i64, 2).normalize())),
                    ..Default::default()
                };

                new_product.insert(&self.ctx.db)
                    .await
                    .map_err(|e| loco_rs::Error::msg(e))?;

                bump_search_version(&self.ctx).await;
                bump_catalog_version(&self.ctx).await;
                tracing::info!("♻️ Catálogo global invalidado en Redis por nuevo producto.");
            }
        }

        tracing::info!("✅ Sincronización finalizada localmente para ID Odoo: {}", args.odoo_id);
        Ok(())
    }
}

async fn bump_search_version(ctx: &AppContext) {
    let ver = ctx
        .cache
        .get::<i64>("products:search_version")
        .await
        .ok()
        .flatten()
        .unwrap_or(0);
    let _ = ctx.cache.insert("products:search_version", &(ver + 1)).await;
}

async fn bump_catalog_version(ctx: &AppContext) {
    let ver = ctx
        .cache
        .get::<i64>("products:catalog_version")
        .await
        .ok()
        .flatten()
        .unwrap_or(0);
    let _ = ctx.cache.insert("products:catalog_version", &(ver + 1)).await;
}
