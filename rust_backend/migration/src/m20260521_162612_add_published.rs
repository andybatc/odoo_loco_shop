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
                    ColumnDef::new(Alias::new("is_published"))
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("products"))
                .drop_column(Alias::new("is_published"))
                .to_owned(),
        )
        .await
    }
}
