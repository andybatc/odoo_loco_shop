use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Alias::new("cart_items"))
                .if_not_exists()
                .col(ColumnDef::new(Alias::new("id")).uuid().not_null().primary_key())
                .col(ColumnDef::new(Alias::new("cart_id")).uuid().not_null())
                .col(ColumnDef::new(Alias::new("product_id")).uuid().not_null())
                .col(ColumnDef::new(Alias::new("quantity")).integer().not_null().default(1))
                // 🔥 Añadimos la relación FK directamente aquí
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-cart_items-cart_id")
                        .from(Alias::new("cart_items"), Alias::new("cart_id"))
                        .to(Alias::new("carts"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(
            Table::drop()
                .table(Alias::new("cart_items"))
                .to_owned()
        ).await
    }
}

