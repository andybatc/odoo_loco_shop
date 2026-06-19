use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE products ADD COLUMN search_vector tsvector
             GENERATED ALWAYS AS (
               to_tsvector('spanish', coalesce(name, '') || ' ' || coalesce(sku, ''))
             ) STORED;"
        ).await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_products_search ON products USING gin(search_vector);"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS idx_products_search;").await?;
        db.execute_unprepared("ALTER TABLE products DROP COLUMN IF EXISTS search_vector;").await?;
        Ok(())
    }
}
