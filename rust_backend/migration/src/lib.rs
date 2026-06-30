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
mod m20260526_000001_product_search;
mod m20260622_000001_fix_cart_items_product_type;
mod m20260622_000002_create_orders;
mod m20260622_000003_add_category_to_products;
mod m20260622_000004_add_role_to_users;
mod m20260622_000005_add_performance_indexes;
mod m20260630_000001_payment_methods;
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
            Box::new(m20260526_000001_product_search::Migration),
            Box::new(m20260622_000001_fix_cart_items_product_type::Migration),
            Box::new(m20260622_000002_create_orders::Migration),
            Box::new(m20260622_000003_add_category_to_products::Migration),
            Box::new(m20260622_000004_add_role_to_users::Migration),
            Box::new(m20260622_000005_add_performance_indexes::Migration),
            Box::new(m20260630_000001_payment_methods::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}