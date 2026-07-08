use loco_rs::prelude::*;
use sea_orm::{Database, EntityTrait, QueryFilter, ColumnTrait, Statement, ActiveValue};
use crate::models::_entities::{products, shipping_rates};
use crate::models::shipping_rates as shipping_model;

pub struct ShippingSyncWorker;

#[async_trait]
impl task::Task for ShippingSyncWorker {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "sync-shipping".to_string(),
            detail: "Sync warehouse locations and shipping rates from Odoo".to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, _vars: &task::Vars) -> Result<()> {
        let odoo_uri = crate::models::_entities::configs::Entity::find()
            .filter(crate::models::_entities::configs::Column::Key.eq("odoo_db_uri"))
            .one(&app_context.db)
            .await?
            .and_then(|c| c.value)
            .unwrap_or_else(|| "postgres://odoo:postgres@localhost:5432/odoo_prod".to_string());

        let odoo_db = Database::connect(&odoo_uri)
            .await
            .map_err(|e| Error::BadRequest(format!("Error conectando a Odoo: {e}")))?;

        let backend = odoo_db.get_database_backend();

        // Sync warehouse locations
        let warehouses = odoo_db
            .query_all(Statement::from_string(
                backend,
                "SELECT pt.id, rc.name AS country, rcs.name AS state, pt.warehouse_latitude, pt.warehouse_longitude
                 FROM product_template pt
                 LEFT JOIN res_country rc ON pt.warehouse_country_id = rc.id
                 LEFT JOIN res_country_state rcs ON pt.warehouse_state_id = rcs.id
                 WHERE pt.sale_ok = true AND pt.warehouse_country_id IS NOT NULL".to_string(),
            ))
            .await
            .unwrap_or_default();

        for row in &warehouses {
            let odoo_id: i32 = row.try_get_by_index(0).unwrap_or(0);
            let country: Option<String> = row.try_get_by_index(1).ok();
            let state: Option<String> = row.try_get_by_index(2).ok();
            let lat: Option<f64> = row.try_get_by_index(3).ok();
            let lng: Option<f64> = row.try_get_by_index(4).ok();

            if let Some(product) = products::Entity::find()
                .filter(products::Column::OdooId.eq(odoo_id))
                .one(&app_context.db)
                .await?
            {
                let mut active: products::ActiveModel = product.into();
                active.warehouse_country = ActiveValue::Set(country);
                active.warehouse_state = ActiveValue::Set(state);
                active.warehouse_lat = ActiveValue::Set(lat);
                active.warehouse_lng = ActiveValue::Set(lng);
                active.update(&app_context.db).await?;
            }
        }

        tracing::info!("Synced {} warehouse locations", warehouses.len());

        // Sync shipping rates
        let rate_rows = odoo_db
            .query_all(Statement::from_string(
                backend,
                "SELECT rc_o.name AS origin_country, rcs_o.name AS origin_state,
                        rc_d.name AS dest_country, rcs_d.name AS dest_state,
                        sr.amount
                 FROM shipping_rate sr
                 JOIN res_country rc_o ON sr.origin_country_id = rc_o.id
                 JOIN res_country_state rcs_o ON sr.origin_state_id = rcs_o.id
                 JOIN res_country rc_d ON sr.dest_country_id = rc_d.id
                 JOIN res_country_state rcs_d ON sr.dest_state_id = rcs_d.id".to_string(),
            ))
            .await
            .unwrap_or_default();

        let models: Vec<shipping_rates::ActiveModel> = rate_rows
            .iter()
            .filter_map(|r| {
                let origin_country: String = r.try_get_by_index(0).ok()?;
                let origin_state: String = r.try_get_by_index(1).ok()?;
                let dest_country: String = r.try_get_by_index(2).ok()?;
                let dest_state: String = r.try_get_by_index(3).ok()?;
                let amount_f64: f64 = r.try_get_by_index(4).ok()?;
                Some(shipping_rates::ActiveModel {
                    origin_country: ActiveValue::Set(origin_country),
                    origin_state: ActiveValue::Set(origin_state),
                    dest_country: ActiveValue::Set(dest_country),
                    dest_state: ActiveValue::Set(dest_state),
                    amount: ActiveValue::Set(Decimal::try_from(amount_f64).unwrap_or(Decimal::ZERO)),
                    ..Default::default()
                })
            })
            .collect();

        let rate_count = models.len();
        if rate_count > 0 {
            shipping_model::replace_all(&app_context.db, models).await?;
        }

        tracing::info!("Synced {} shipping rates", rate_count);
        Ok(())
    }
}
