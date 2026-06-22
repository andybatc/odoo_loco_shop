use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Alias::new("orders"))
                .if_not_exists()
                .col(ColumnDef::new(Alias::new("id")).uuid().not_null().primary_key())
                .col(ColumnDef::new(Alias::new("user_id")).integer().null())
                .col(ColumnDef::new(Alias::new("customer_name")).string().not_null())
                .col(ColumnDef::new(Alias::new("customer_email")).string().not_null())
                .col(ColumnDef::new(Alias::new("customer_phone")).string().null())
                .col(ColumnDef::new(Alias::new("customer_street")).string().null())
                .col(ColumnDef::new(Alias::new("customer_city")).string().null())
                .col(ColumnDef::new(Alias::new("customer_zip")).string().null())
                .col(ColumnDef::new(Alias::new("odoo_order_name")).string().null())
                .col(ColumnDef::new(Alias::new("odoo_invoice_name")).string().null())
                .col(ColumnDef::new(Alias::new("total")).decimal().not_null())
                .col(ColumnDef::new(Alias::new("status")).string().not_null().default("pending"))
                .col(ColumnDef::new(Alias::new("created_at")).timestamp().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Alias::new("updated_at")).timestamp().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-orders-user_id")
                        .from(Alias::new("orders"), Alias::new("user_id"))
                        .to(Alias::new("users"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::SetNull)
                )
                .to_owned()
        ).await?;

        m.create_table(
            Table::create()
                .table(Alias::new("order_items"))
                .if_not_exists()
                .col(ColumnDef::new(Alias::new("id")).uuid().not_null().primary_key())
                .col(ColumnDef::new(Alias::new("order_id")).uuid().not_null())
                .col(ColumnDef::new(Alias::new("product_id")).integer().not_null())
                .col(ColumnDef::new(Alias::new("product_name")).string().not_null())
                .col(ColumnDef::new(Alias::new("price")).decimal().not_null())
                .col(ColumnDef::new(Alias::new("quantity")).integer().not_null())
                .col(ColumnDef::new(Alias::new("subtotal")).decimal().not_null())
                .col(ColumnDef::new(Alias::new("created_at")).timestamp().not_null().default(Expr::current_timestamp()))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-order_items-order_id")
                        .from(Alias::new("order_items"), Alias::new("order_id"))
                        .to(Alias::new("orders"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-order_items-product_id")
                        .from(Alias::new("order_items"), Alias::new("product_id"))
                        .to(Alias::new("products"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::SetNull)
                )
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Alias::new("order_items")).to_owned()).await?;
        m.drop_table(Table::drop().table(Alias::new("orders")).to_owned()).await?;
        Ok(())
    }
}
