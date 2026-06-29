use loco_rs::prelude::*;
use crate::models::_entities::{carts, cart_items};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub struct CleanupCarts;

#[async_trait]
impl task::Task for CleanupCarts {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "cleanup-carts".to_string(),
            detail: "Delete abandoned carts older than 7 days".to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(7);

        let old_carts = carts::Entity::find()
            .filter(carts::Column::UpdatedAt.lt(cutoff))
            .all(&ctx.db)
            .await?;

        let count = old_carts.len();
        if count == 0 {
            tracing::info!("No hay carritos abandonados para limpiar.");
            return Ok(());
        }

        for cart in &old_carts {
            cart_items::Entity::delete_many()
                .filter(cart_items::Column::CartId.eq(cart.id))
                .exec(&ctx.db)
                .await?;
        }

        let ids: Vec<_> = old_carts.iter().map(|c| c.id).collect();
        carts::Entity::delete_many()
            .filter(carts::Column::Id.is_in(ids))
            .exec(&ctx.db)
            .await?;

        tracing::info!("Carritos abandonados eliminados: {count}");
        Ok(())
    }
}
