use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::_entities::shipping_rates;

pub async fn find_rate(
    db: &DatabaseConnection,
    origin_country: &str,
    origin_state: &str,
    dest_country: &str,
    dest_state: &str,
) -> Result<Option<Decimal>, DbErr> {
    shipping_rates::Entity::find()
        .filter(shipping_rates::Column::OriginCountry.eq(origin_country))
        .filter(shipping_rates::Column::OriginState.eq(origin_state))
        .filter(shipping_rates::Column::DestCountry.eq(dest_country))
        .filter(shipping_rates::Column::DestState.eq(dest_state))
        .one(db)
        .await
        .map(|r| r.map(|m| m.amount))
}

pub async fn find_rate_by_country(
    db: &DatabaseConnection,
    origin_country: &str,
    dest_country: &str,
    dest_state: &str,
) -> Result<Option<Decimal>, DbErr> {
    shipping_rates::Entity::find()
        .filter(shipping_rates::Column::OriginCountry.eq(origin_country))
        .filter(shipping_rates::Column::DestCountry.eq(dest_country))
        .filter(shipping_rates::Column::DestState.eq(dest_state))
        .one(db)
        .await
        .map(|r| r.map(|m| m.amount))
}

pub async fn replace_all(
    db: &DatabaseConnection,
    rates: Vec<shipping_rates::ActiveModel>,
) -> Result<(), DbErr> {
    shipping_rates::Entity::delete_many().exec(db).await?;
    for rate in rates {
        rate.insert(db).await?;
    }
    Ok(())
}
