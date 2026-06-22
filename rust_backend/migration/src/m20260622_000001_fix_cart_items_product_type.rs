use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // 1. Drop old UUID column
        m.alter_table(
            Table::alter()
                .table(Alias::new("cart_items"))
                .drop_column(Alias::new("product_id"))
                .to_owned()
        ).await?;

        // 2. Add new INTEGER column with default 1 for existing rows
        m.alter_table(
            Table::alter()
                .table(Alias::new("cart_items"))
                .add_column(
                    ColumnDef::new(Alias::new("product_id"))
                        .integer()
                        .not_null()
                        .default(1)
                )
                .to_owned()
        ).await?;

        // 3. Add FK constraint to products.id
        m.create_foreign_key(
            ForeignKey::create()
                .name("fk-cart_items-product_id")
                .from(Alias::new("cart_items"), Alias::new("product_id"))
                .to(Alias::new("products"), Alias::new("id"))
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_foreign_key(
            ForeignKey::drop()
                .name("fk-cart_items-product_id")
                .table(Alias::new("cart_items"))
                .to_owned()
        ).await?;

        m.alter_table(
            Table::alter()
                .table(Alias::new("cart_items"))
                .drop_column(Alias::new("product_id"))
                .to_owned()
        ).await?;

        m.alter_table(
            Table::alter()
                .table(Alias::new("cart_items"))
                .add_column(
                    ColumnDef::new(Alias::new("product_id"))
                        .uuid()
                        .not_null()
                )
                .to_owned()
        ).await?;

        Ok(())
    }
}
