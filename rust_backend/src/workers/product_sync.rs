use serde::{Deserialize, Serialize};
use loco_rs::prelude::*;
use crate::models::product_template_odoo;
use crate::models::_entities::{configs, products};
use sea_orm::{Database, sea_query::OnConflict};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {
}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "ProductSync".to_string()
    }

    async fn perform(&self, _args: WorkerArgs) -> Result<()> {
        tracing::info!("Iniciando sincronización: Odoo -> Local");

        let odoo_uri = configs::Entity::find()
            .filter(configs::Column::Key.eq("odoo_db_uri"))
            .one(&self.ctx.db)
            .await?
            .and_then(|c| c.value)
            .unwrap_or_else(|| "postgres://odoo:postgres@localhost:5432/odoo_prod".to_string());

        let odoo_db = Database::connect(&odoo_uri)
            .await
            .map_err(|e| Error::BadRequest(format!("Error conectando a Odoo: {e}")))?;

        let category_map: std::collections::HashMap<i32, String> = {
            use sea_orm::Statement;
            let rows = odoo_db
                .query_all(Statement::from_string(
                    odoo_db.get_database_backend(),
                    "SELECT id, name FROM product_category".to_string(),
                ))
                .await
                .unwrap_or_default();
            rows.into_iter()
                .filter_map(|r| {
                    let id: i32 = r.try_get_by_index::<i32>(0).ok()?;
                    let name: Option<String> = r.try_get_by_index::<String>(1).ok();
                    name.map(|n| (id, n))
                })
                .collect()
        };

        let odoo_products = product_template_odoo::Entity::find()
            .filter(product_template_odoo::Column::IsPublished.eq(true))
            .all(&odoo_db)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        tracing::info!("Se encontraron {} productos en Odoo.", odoo_products.len());

        for item in odoo_products {
            let name_string = item.name.get("es_ES")
                .or(item.name.get("en_US"))
                .and_then(|v| v.as_str())
                .unwrap_or("Sin nombre");

            tracing::info!("Procesando: {} (ID Odoo: {})", name_string, item.id);

            let category_name = category_map.get(&item.categ_id).cloned();

            let active_product = products::ActiveModel {
                odoo_id: Set(Some(item.id)),
                name: Set(Some(name_string.to_string())),
                sku: Set(item.default_code.clone()),
                price: Set(Some(item.list_price.unwrap_or_default())),
                stock: Set(Some(0.0)),
                category: Set(category_name),
                ..Default::default()
            };

            match products::Entity::insert(active_product)
                .on_conflict(
                    OnConflict::column(products::Column::OdooId)
                        .update_columns([
                            products::Column::Name,
                            products::Column::Sku,
                            products::Column::Price,
                            products::Column::Category,
                        ])
                        .to_owned()
                )
                .exec(&self.ctx.db)
                .await
            {
                Ok(res) => tracing::info!("Guardado exitoso (ID Local: {:?})", res.last_insert_id),
                Err(err) => tracing::error!("Error guardando {}: {}", name_string, err),
            }
        }

        tracing::info!("Sincronización completada.");
        Ok(())
    }
}