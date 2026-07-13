use crate::models::_entities::shipping_rates;
use loco_rs::bgworker::BackgroundWorker;
use loco_rs::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct RatePayload {
    pub origin_country: String,
    pub origin_state: String,
    pub dest_country: String,
    pub dest_state: String,
    pub amount: f64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ShippingRateSyncArgs {
    pub rates: Vec<RatePayload>,
}

pub struct ShippingRateSyncWorker {
    pub ctx: AppContext,
}

#[async_trait]
impl BackgroundWorker<ShippingRateSyncArgs> for ShippingRateSyncWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "ShippingRateSync".to_string()
    }

    async fn perform(&self, args: ShippingRateSyncArgs) -> Result<()> {
        let now = chrono::Utc::now().into();
        for rate in &args.rates {
            let existing = shipping_rates::Entity::find()
                .filter(shipping_rates::Column::OriginCountry.eq(&rate.origin_country))
                .filter(shipping_rates::Column::OriginState.eq(&rate.origin_state))
                .filter(shipping_rates::Column::DestCountry.eq(&rate.dest_country))
                .filter(shipping_rates::Column::DestState.eq(&rate.dest_state))
                .one(&self.ctx.db)
                .await?;

            let amount =
                sea_orm::prelude::Decimal::try_from(rate.amount).unwrap_or(sea_orm::prelude::Decimal::ZERO);

            if let Some(existing) = existing {
                let mut am: shipping_rates::ActiveModel = existing.into();
                am.amount = Set(amount);
                am.updated_at = Set(now);
                am.update(&self.ctx.db).await?;
            } else {
                shipping_rates::ActiveModel {
                    origin_country: Set(rate.origin_country.clone()),
                    origin_state: Set(rate.origin_state.clone()),
                    dest_country: Set(rate.dest_country.clone()),
                    dest_state: Set(rate.dest_state.clone()),
                    amount: Set(amount),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&self.ctx.db)
                .await?;
            }
        }
        tracing::info!("Synced {} shipping rates via worker", args.rates.len());
        Ok(())
    }
}
