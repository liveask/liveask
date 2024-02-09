mod error;

use std::str::FromStr;

use stripe::{
    CheckoutSession, CheckoutSessionId, CheckoutSessionMode, CheckoutSessionStatus, Client,
    CreateCheckoutSession, CreateCheckoutSessionLineItems, ListProducts,
};

pub use self::error::PaymentError;
use self::error::PaymentResult;

#[derive(Clone)]
pub struct Payment {
    client: Client,
    price: Option<String>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
impl Default for Payment {
    fn default() -> Self {
        Self {
            client: Client::new(String::new()),
            price: None,
        }
    }
}

impl Payment {
    pub fn new(secret: String) -> Self {
        let client = Client::new(secret);
        Self {
            client,
            price: None,
        }
    }

    pub async fn authenticate(&mut self, live: bool) -> PaymentResult<String> {
        let res = stripe::Product::list(&self.client, &ListProducts::new()).await?;

        for p in &res.data {
            tracing::info!(
                "[stripe] prod: {:?} [id: {}, live: {:?}, active: {:?}, price: {:?}]",
                p.name,
                p.id,
                p.livemode,
                p.active,
                p.default_price,
            );

            if p.metadata
                .as_ref()
                .and_then(|meta| meta.get("id"))
                .is_some_and(|id| id == "premium")
            {
                tracing::info!("[stripe] prod id: {:?} is premium package", p.id);

                if p.livemode.unwrap_or_default() == live {
                    self.price = Some(
                        p.default_price
                            .as_ref()
                            .ok_or_else(|| PaymentError::Generic("default price not set".into()))?
                            .id()
                            .to_string(),
                    );

                    return Ok(p.id.to_string());
                }
            }
        }

        Err(PaymentError::Generic(String::from(
            "no premium product found",
        )))
    }

    pub async fn create_order(
        &self,
        event: &str,
        mod_url: &str,
        return_url: &str,
    ) -> PaymentResult<String> {
        let checkout_session = {
            let mut params = CreateCheckoutSession::new();
            params.cancel_url = Some(mod_url);
            params.success_url = Some(return_url);
            params.client_reference_id = Some(event);
            params.allow_promotion_codes = Some(true);
            params.metadata = Some(
                vec![
                    (String::from("event"), event.to_string()),
                    (String::from("url"), mod_url.to_string()),
                ]
                .into_iter()
                .collect(),
            );
            params.mode = Some(CheckoutSessionMode::Payment);
            params.line_items = Some(vec![CreateCheckoutSessionLineItems {
                quantity: Some(1),
                price: Some(
                    self.price
                        .clone()
                        .ok_or_else(|| PaymentError::Generic("price id not defined".into()))?,
                ),
                ..Default::default()
            }]);

            CheckoutSession::create(&self.client, params).await?
        };

        checkout_session
            .url
            .ok_or_else(|| PaymentError::Generic("no url in payment session".into()))
    }

    pub async fn retrieve_event_state(&self, session_id: String) -> PaymentResult<(String, bool)> {
        let sess = CheckoutSessionId::from_str(session_id.as_str())?;

        let sess = CheckoutSession::retrieve(&self.client, &sess, &[]).await?;

        let event = sess.client_reference_id.unwrap_or_default();
        let completed = sess
            .status
            .is_some_and(|status| status == CheckoutSessionStatus::Complete);

        Ok((event, completed))
    }
}
