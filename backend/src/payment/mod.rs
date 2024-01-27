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
    pub fn new(secret: String) -> PaymentResult<Self> {
        let client = Client::new(secret);
        Ok(Self {
            client,
            price: None,
        })
    }

    pub async fn authenticate(&mut self) -> PaymentResult<bool> {
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
                .map(|id| id == "premium")
                .unwrap_or_default()
            {
                tracing::info!("[stripe] prod id: {:?} is premium package", p.id);
                self.price = Some(p.default_price.as_ref().unwrap().id().to_string());

                if p.livemode.unwrap() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
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
            params.mode = Some(CheckoutSessionMode::Payment);
            params.line_items = Some(vec![CreateCheckoutSessionLineItems {
                quantity: Some(1),
                price: Some(self.price.clone().unwrap()),
                ..Default::default()
            }]);

            CheckoutSession::create(&self.client, params).await.unwrap()
        };

        Ok(checkout_session.url.unwrap())
    }

    pub async fn retrieve_event_state(&self, session_id: String) -> PaymentResult<(String, bool)> {
        let sess = CheckoutSessionId::from_str(session_id.as_str())?;

        let sess = CheckoutSession::retrieve(&self.client, &sess, &[]).await?;

        let event = sess.client_reference_id.unwrap_or_default();
        let completed = sess
            .status
            .map(|status| status == CheckoutSessionStatus::Complete)
            .unwrap_or_default();

        Ok((event, completed))
    }
}
