#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::models::_entities::orders;

static confirmed: Dir<'_> = include_dir!("src/mailers/order/confirmed");

pub struct OrderMailer {}
impl Mailer for OrderMailer {}
impl OrderMailer {
    pub async fn send_confirmation(ctx: &AppContext, order: &orders::Model) -> Result<()> {
        Self::mail_template(
            ctx,
            &confirmed,
            mailer::Args {
                to: order.customer_email.clone(),
                locals: json!({
                    "customer_name": order.customer_name,
                    "order_name": order.odoo_order_name,
                    "total": order.total,
                    "status": order.status,
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
