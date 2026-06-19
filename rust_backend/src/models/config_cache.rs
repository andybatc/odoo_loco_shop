use loco_rs::prelude::*;
use crate::models::_entities::configs;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::time::Duration;

pub async fn get_cached_config(ctx: &AppContext, key: &str) -> Result<Option<String>> {
    let cache_key = format!("config:{}", key);

    if let Ok(Some(val)) = ctx.cache.get::<String>(&cache_key).await {
        return Ok(Some(val));
    }

    let config = configs::Entity::find()
        .filter(configs::Column::Key.eq(key))
        .one(&ctx.db)
        .await?;

    let val = config.and_then(|c| c.value);
    if let Some(ref v) = val {
        let _ = ctx
            .cache
            .insert_with_expiry(&cache_key, v, Duration::from_secs(60))
            .await;
    }

    Ok(val)
}

pub async fn invalidate_config_cache(ctx: &AppContext, key: &str) {
    let cache_key = format!("config:{}", key);
    let _ = ctx.cache.remove(&cache_key).await;
}
