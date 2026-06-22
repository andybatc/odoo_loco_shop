use loco_rs::prelude::ModelResult;
use sea_orm::entity::prelude::*;
use sea_orm::{DbBackend, Statement, ConnectionTrait};
pub use super::_entities::products::{ActiveModel, Model, Entity};
pub type Products = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !self.price.is_unchanged() {
            if let Some(price) = self.price.clone().unwrap() {
                if price < sea_orm::prelude::Decimal::ZERO {
                    return Err(DbErr::Custom("price cannot be negative".to_string()));
                }
            }
        }
        if !self.name.is_unchanged() {
            if let Some(name) = self.name.clone().unwrap() {
                if name.trim().is_empty() {
                    return Err(DbErr::Custom("name cannot be empty".to_string()));
                }
            }
        }
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {}

impl ActiveModel {}

impl Entity {
    pub async fn search_products(
        db: &DatabaseConnection,
        query: &str,
        page: u32,
        page_size: u32,
    ) -> ModelResult<(Vec<Model>, u64)> {
        if query.trim().is_empty() {
            return Ok((Vec::new(), 0));
        }
        let offset = (page.saturating_sub(1)) * page_size;

        let count_result = db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"SELECT COUNT(*) FROM products
                   WHERE search_vector @@ plainto_tsquery('spanish', $1)
                   AND is_published = true"#,
                vec![query.into()],
            ))
            .await?;

        let total: i64 = count_result
            .and_then(|r| r.try_get_by_index::<i64>(0).ok())
            .unwrap_or(0);

        let results = Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"SELECT id, odoo_id, sku, name, price, stock, image_filename,
                          is_published, created_at, updated_at
                   FROM products
                   WHERE search_vector @@ plainto_tsquery('spanish', $1)
                   AND is_published = true
                   ORDER BY ts_rank(search_vector, plainto_tsquery('spanish', $2)) DESC
                   LIMIT $3 OFFSET $4"#,
                vec![query.into(), query.into(), page_size.into(), offset.into()],
            ))
            .all(db)
            .await?;

        Ok((results, total as u64))
    }
}
