#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20260506_184955_products;
mod m20260506_194459_add_unique_to_odoo_id;
mod m20260507_200140_configs;
mod m20260520_153037_add_image_to_products;
mod m20260521_162612_add_published;
mod m20260524_140919_carts;
mod m20260524_140942_create_cart_items;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20260506_184955_products::Migration),
            Box::new(m20260506_194459_add_unique_to_odoo_id::Migration),
            Box::new(m20260507_200140_configs::Migration),
            Box::new(m20260520_153037_add_image_to_products::Migration),
            Box::new(m20260521_162612_add_published::Migration),
            Box::new(m20260524_140919_carts::Migration),
            Box::new(m20260524_140942_create_cart_items::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}