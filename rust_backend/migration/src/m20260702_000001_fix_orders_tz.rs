use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // orders.created_at y orders.updated_at
        m.alter_table(
            Table::alter()
                .table(Alias::new("orders"))
                .modify_column(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .modify_column(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        // order_items.created_at
        m.alter_table(
            Table::alter()
                .table(Alias::new("order_items"))
                .modify_column(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("orders"))
                .modify_column(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .modify_column(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Alias::new("order_items"))
                .modify_column(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await
    }
}
