use sea_orm::entity::prelude::*;
use sea_orm::QueryOrder;
pub use super::_entities::payment_methods::{ActiveModel, Model, Entity};
pub type PaymentMethods = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {}

impl ActiveModel {}

impl Entity {}

pub async fn find_all_active(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
    Entity::find()
        .filter(super::_entities::payment_methods::Column::IsActive.eq(true))
        .order_by_asc(super::_entities::payment_methods::Column::Sequence)
        .all(db)
        .await
}
