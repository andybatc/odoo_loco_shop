use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_index(
            Index::create()
                .name("idx-configs-key")
                .table(Alias::new("configs"))
                .col(Alias::new("key"))
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-users-pid")
                .table(Alias::new("users"))
                .col(Alias::new("pid"))
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-cart-items-cart-id")
                .table(Alias::new("cart_items"))
                .col(Alias::new("cart_id"))
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-cart-items-product-id")
                .table(Alias::new("cart_items"))
                .col(Alias::new("product_id"))
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-orders-status")
                .table(Alias::new("orders"))
                .col(Alias::new("status"))
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-orders-created-at")
                .table(Alias::new("orders"))
                .col(Alias::new("created_at"))
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx-carts-user-id")
                .table(Alias::new("carts"))
                .col(Alias::new("user_id"))
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(Index::drop().name("idx-configs-key").table(Alias::new("configs")).to_owned())
            .await?;
        m.drop_index(Index::drop().name("idx-users-pid").table(Alias::new("users")).to_owned())
            .await?;
        m.drop_index(Index::drop().name("idx-cart-items-cart-id").table(Alias::new("cart_items")).to_owned())
            .await?;
        m.drop_index(Index::drop().name("idx-cart-items-product-id").table(Alias::new("cart_items")).to_owned())
            .await?;
        m.drop_index(Index::drop().name("idx-orders-status").table(Alias::new("orders")).to_owned())
            .await?;
        m.drop_index(Index::drop().name("idx-orders-created-at").table(Alias::new("orders")).to_owned())
            .await?;
        m.drop_index(Index::drop().name("idx-carts-user-id").table(Alias::new("carts")).to_owned())
            .await
    }
}
