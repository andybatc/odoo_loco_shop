use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Alias::new("carts"))
                .if_not_exists()
                // Definimos 'id' explícitamente como Llave Primaria (Primary Key)
                .col(ColumnDef::new(Alias::new("id")).uuid().not_null().primary_key())
                // user_id puede ser nulo para carritos de invitados
                .col(ColumnDef::new(Alias::new("user_id")).uuid().null())
                // Añadimos timestamps (muy útiles para borrar carritos abandonados en el futuro)
                .col(ColumnDef::new(Alias::new("created_at")).timestamp().not_null().default(Expr::current_timestamp()))
                .col(ColumnDef::new(Alias::new("updated_at")).timestamp().not_null().default(Expr::current_timestamp()))
                .to_owned()
        ).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "carts").await
    }
}
