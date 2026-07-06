use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("users"))
                .add_column(ColumnDef::new(Alias::new("phone")).string().null())
                .add_column(ColumnDef::new(Alias::new("street")).string().null())
                .add_column(ColumnDef::new(Alias::new("city")).string().null())
                .add_column(ColumnDef::new(Alias::new("zip")).string().null())
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("users"))
                .drop_column(Alias::new("phone"))
                .drop_column(Alias::new("street"))
                .drop_column(Alias::new("city"))
                .drop_column(Alias::new("zip"))
                .to_owned(),
        )
        .await
    }
}
