use axum::{
    extract::{FromRequest, State},
    http::Request,
    response::{Html, IntoResponse, Response},
};
use reqwest::StatusCode;
use stripe::{Event, EventObject, EventType};

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

pub async fn handle_webhook(
    State(app): State<SharedApp>,
    StripeEvent(event): StripeEvent,
) -> std::result::Result<impl IntoResponse, InternalError> {
    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            if let EventObject::CheckoutSession(session) = event.data.object {
                tracing::info!("[hooks] CheckoutSessionCompleted: {:?}", session.id);

                if let Some(event) = session.client_reference_id {
                    if let Err(e) = app.payment_webhook(session.id.to_string(), event).await {
                        tracing::error!("[hooks] failed: {e}");
                    }
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
