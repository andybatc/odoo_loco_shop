use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m
            .alter_table(
                Table::alter()
                    .table(Alias::new("products")) // Asegúrate que el nombre de la tabla coincida
                    .add_column(
                        ColumnDef::new(Alias::new("image_filename"))
                            .string()
                            .null() // Permite que sea opcional
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _m: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
