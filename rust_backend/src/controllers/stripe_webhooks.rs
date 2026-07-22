#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::controllers::checkout;
use loco_rs::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct StripeEvent {
    #[serde(rename = "type")]
    event_type: String,
    id: String,
    data: StripeEventData,
}

#[derive(Debug, Deserialize)]
struct StripeEventData {
    object: serde_json::Value,
}

#[debug_handler]
pub async fn handle_stripe_webhook(
    State(ctx): State<AppContext>,
    Json(event): Json<StripeEvent>,
) -> Result<Response> {
    tracing::info!(
        "Stripe webhook received: type={}, id={}",
        event.event_type,
        event.id
    );

    match event.event_type.as_str() {
        "checkout.session.completed" => {
            let session = &event.data.object;
            let session_id = session["id"].as_str().unwrap_or("");

            if session_id.is_empty() {
                tracing::error!("Stripe webhook: checkout.session.completed without session id");
                return format::json(serde_json::json!({"status": "received"}));
            }

            if session["payment_status"].as_str() != Some("paid") {
                tracing::warn!(
                    "Stripe webhook: session {} not paid (status: {:?}), skipping",
                    session_id,
                    session["payment_status"].as_str()
                );
                return format::json(serde_json::json!({"status": "received"}));
            }

            match checkout::process_paid_session(&ctx, session_id, None).await {
                Ok((order_id, total, already_processed)) => {
                    if already_processed {
                        tracing::info!(
                            "Stripe webhook: session {} already processed (order: {})",
                            session_id,
                            order_id
                        );
                    } else {
                        tracing::info!(
                            "Stripe webhook: order {} created for session {}, total={}",
                            order_id,
                            session_id,
                            total
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Stripe webhook: failed to process session {}: {:?}",
                        session_id,
                        e
                    );
                }
            }
        }
        other => {
            tracing::debug!("Stripe webhook: unhandled event type {}", other);
        }
    }

    format::json(serde_json::json!({"status": "received"}))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/webhooks")
        .add("/stripe", post(handle_stripe_webhook))
}
