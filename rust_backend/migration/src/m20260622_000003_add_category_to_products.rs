use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("products"))
                .add_column(
                    ColumnDef::new(Alias::new("category"))
                        .string()
                        .null()
                )
                .to_owned()
        ).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("products"))
                .drop_column(Alias::new("category"))
                .to_owned()
        ).await
    }
}
