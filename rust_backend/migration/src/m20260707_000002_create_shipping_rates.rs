use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShippingRates::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ShippingRates::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(ShippingRates::OriginCountry).string().not_null())
                    .col(ColumnDef::new(ShippingRates::OriginState).string().not_null())
                    .col(ColumnDef::new(ShippingRates::DestCountry).string().not_null())
                    .col(ColumnDef::new(ShippingRates::DestState).string().not_null())
                    .col(ColumnDef::new(ShippingRates::Amount).decimal().not_null())
                    .col(ColumnDef::new(ShippingRates::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(ShippingRates::UpdatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ShippingRates::Table)
                    .unique()
                    .col(ShippingRates::OriginCountry)
                    .col(ShippingRates::OriginState)
                    .col(ShippingRates::DestCountry)
                    .col(ShippingRates::DestState)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ShippingRates::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum ShippingRates {
    Table,
    Id,
    OriginCountry,
    OriginState,
    DestCountry,
    DestState,
    Amount,
    CreatedAt,
    UpdatedAt,
}
