use sea_orm::entity::prelude::*;
pub use super::_entities::order_items::{ActiveModel, Model, Entity};
pub type OrderItems = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, _insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !self.quantity.is_unchanged() && self.quantity.clone().unwrap() <= 0 {
            return Err(DbErr::Custom("quantity must be greater than 0".to_string()));
        }
        Ok(self)
    }
}

impl Model {}

impl ActiveModel {}

impl Entity {}
