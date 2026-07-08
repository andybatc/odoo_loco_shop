use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseCountry).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseState).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseLat).double().null())
                    .add_column_if_not_exists(ColumnDef::new(Products::WarehouseLng).double().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_column(Products::WarehouseCountry)
                    .drop_column(Products::WarehouseState)
                    .drop_column(Products::WarehouseLat)
                    .drop_column(Products::WarehouseLng)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Products {
    Table,
    WarehouseCountry,
    WarehouseState,
    WarehouseLat,
    WarehouseLng,
}
