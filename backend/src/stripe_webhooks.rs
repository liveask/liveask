use axum::{
    extract::{FromRequest, State},
    http::Request,
    response::{Html, IntoResponse, Response},
};
use reqwest::StatusCode;
use stripe::{CheckoutSessionPaymentStatus, Event, EventObject, EventType};

use crate::{app::SharedApp, env, error::InternalError};

pub struct StripeEvent(Event);

#[async_trait::async_trait]
impl<S, B> FromRequest<S, B> for StripeEvent
where
    String: FromRequest<S, B>,
    B: Send + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let signature = if let Some(sig) = req.headers().get("stripe-signature") {
            sig.to_owned()
        } else {
            return Err(StatusCode::BAD_REQUEST.into_response());
        };

        let payload = String::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;

        //TODO: do not read env everytime
        let secret = std::env::var(env::ENV_STRIPE_HOOK_SECRET).unwrap_or_default();

        // Fail closed: an empty signing secret makes the webhook HMAC publicly computable,
        // so anyone could forge a paid checkout. Never verify against an empty key.
        if secret.is_empty() {
            tracing::error!("stripe webhook secret not configured; rejecting webhook");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        Ok(Self(
            stripe::Webhook::construct_event(
                &payload,
                signature.to_str().unwrap_or_default(),
                &secret,
            )
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?,
        ))
    }
}

// TODO: cleanup
#[allow(clippy::cognitive_complexity)]
pub async fn handle_webhook(
    State(app): State<SharedApp>,
    StripeEvent(event): StripeEvent,
) -> std::result::Result<impl IntoResponse, InternalError> {
    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            if let EventObject::CheckoutSession(session) = event.data.object {
                tracing::info!(
                    "[hooks] CheckoutSessionCompleted: {:?} (payment_status: {:?})",
                    session.id,
                    session.payment_status
                );

                // a completed session can still be `Unpaid` for async/delayed payment
                // methods; only fulfil once Stripe reports the money actually cleared
                if matches!(
                    session.payment_status,
                    CheckoutSessionPaymentStatus::Paid
                        | CheckoutSessionPaymentStatus::NoPaymentRequired
                ) {
                    if let Some(event) = session.client_reference_id {
                        // propagate so a transient failure returns 5xx and Stripe retries,
                        // instead of silently dropping an upgrade the customer paid for
                        app.payment_webhook(session.id.to_string(), event).await?;
                    }
                } else {
                    tracing::warn!(
                        "[hooks] checkout completed but not paid ({:?}); skipping upgrade",
                        session.payment_status
                    );
                }
            }
        }
        //TODO: handle refunds
        _ => {
            tracing::warn!("[hooks] unknown stripe hook: {:?}", event.type_);
        }
    }

    Ok(Html(""))
}
