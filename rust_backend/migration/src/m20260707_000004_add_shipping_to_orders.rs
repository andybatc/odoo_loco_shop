use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Orders::Table)
                    .add_column_if_not_exists(ColumnDef::new(Orders::ShippingCost).decimal().null())
                    .add_column_if_not_exists(ColumnDef::new(Orders::CustomerCountry).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Orders::CustomerState).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Orders::Table)
                    .drop_column(Orders::ShippingCost)
                    .drop_column(Orders::CustomerCountry)
                    .drop_column(Orders::CustomerState)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Orders {
    Table,
    ShippingCost,
    CustomerCountry,
    CustomerState,
}
