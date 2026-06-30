use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Alias::new("payment_methods"))
                .if_not_exists()
                .col(
                    ColumnDef::new(Alias::new("id"))
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(Alias::new("odoo_provider_id"))
                        .integer()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Alias::new("name")).string().not_null())
                .col(ColumnDef::new(Alias::new("code")).string().not_null())
                .col(
                    ColumnDef::new(Alias::new("is_active"))
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(Alias::new("is_published"))
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(Alias::new("allow_tokenization"))
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(Alias::new("capture_manually"))
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(Alias::new("sequence"))
                        .integer()
                        .not_null()
                        .default(0),
                )
                .col(
                    ColumnDef::new(Alias::new("created_at"))
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Alias::new("updated_at"))
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Alias::new("payment_methods")).to_owned())
            .await?;
        Ok(())
    }
}
