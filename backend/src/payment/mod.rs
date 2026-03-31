mod error;

use futures_util::TryStreamExt;
use std::str::FromStr;
use stripe::{
    CheckoutSession, CheckoutSessionId, CheckoutSessionMode, CheckoutSessionStatus, Client,
    CreateCheckoutSession, CreateCheckoutSessionLineItems, Customer, CustomerId, ListProducts,
    ListSubscriptions, SubscriptionStatus,
};
use tracing::instrument;

pub use self::error::PaymentError;
use self::error::PaymentResult;

#[derive(Clone)]
pub struct Payment {
    client: Client,
    price: Option<String>,
    subscription_url: Option<String>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
impl Default for Payment {
    fn default() -> Self {
        Self {
            client: Client::new(String::new()),
            price: None,
            subscription_url: None,
        }
    }
}

impl Payment {
    pub fn new(secret: String) -> Self {
        let client = Client::new(secret);
        Self {
            client,
            price: None,
            subscription_url: None,
        }
    }

    pub async fn authenticate(&mut self, live: bool) -> PaymentResult<String> {
        let res = stripe::Product::list(&self.client, &ListProducts::new()).await?;

        let mut premium_product: Option<String> = None;

        for p in &res.data {
            tracing::info!(
                "[stripe] prod: {:?} [id: {}, live: {:?}, active: {:?}, price: {:?}]",
                p.name,
                p.id,
                p.livemode,
                p.active,
                p.default_price,
            );

            if p.livemode.unwrap_or_default() != live {
                continue;
            }

            if let Some(meta) = &p.metadata {
                if meta.get("id").is_some_and(|id| id == "premium") {
                    tracing::info!("[stripe] prod id: {:?} is premium package", p.id);

                    self.price = Some(
                        p.default_price
                            .as_ref()
                            .ok_or_else(|| PaymentError::Generic("default price not set".into()))?
                            .id()
                            .to_string(),
                    );
                    premium_product = Some(p.id.to_string());
                }
            }
        }

        // Fetch payment links via raw HTTP request (avoiding deserialization issues)
        match self.fetch_payment_link_url().await {
            Ok(Some(url)) => {
                self.subscription_url = Some(url.clone());
                tracing::info!("[stripe] using subscription payment link: {}", url);
            }
            Ok(None) => {
                tracing::warn!("[stripe] no active payment links found for subscription");
            }
            Err(e) => {
                tracing::error!("[stripe] failed to fetch payment links: {:?}", e);
            }
        }

        premium_product
            .ok_or_else(|| PaymentError::Generic(String::from("no premium product found")))
    }

    #[instrument(skip(self))]
    async fn fetch_payment_link_url(&self) -> PaymentResult<Option<String>> {
        use serde_json::Value;

        let response = self.client.get("/payment_links").await?;

        if let Value::Object(obj) = response {
            if let Some(Value::Array(data)) = obj.get("data") {
                tracing::info!("[stripe] found {} payment links", data.len());

                for item in data {
                    if let Value::Object(link) = item {
                        // Check if active
                        if let Some(Value::Bool(true)) = link.get("active") {
                            // Get URL
                            if let Some(Value::String(url)) = link.get("url") {
                                return Ok(Some(url.clone()));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub fn subscription_url(&self) -> PaymentResult<&str> {
        self.subscription_url
            .as_deref()
            .ok_or_else(|| PaymentError::Generic(String::from("subscription url not found")))
    }

    #[instrument(skip(self))]
    pub async fn subscription_checkout(&self, checkout: String) -> PaymentResult<String> {
        let sess = CheckoutSessionId::from_str(checkout.as_str())?;

        let sess = CheckoutSession::retrieve(&self.client, &sess, &[]).await?;

        if let Some(customer) = sess.customer {
            let id = customer.id();

            return Ok(id.as_str().to_string());
        }

        Err(PaymentError::Generic(String::from("no customer found")))
    }

    #[instrument(skip(self))]
    pub async fn customer_email(&self, customer: &str) -> PaymentResult<Option<String>> {
        let id = CustomerId::from_str(customer)?;
        let customer = Customer::retrieve(&self.client, &id, &[]).await?;
        Ok(customer.email)
    }

    #[instrument(skip(self))]
    pub async fn verify_customer(&self, customer_id: &str) -> PaymentResult<String> {
        tracing::info!("verify_customer");
        let id = CustomerId::from_str(customer_id)?;
        let customer = Customer::retrieve(&self.client, &id, &["subscriptions"]).await?;
        tracing::info!("customer: {customer:?}");

        let list = ListSubscriptions::new();
        let mut subscriptions = customer.subscriptions.paginate(list).stream(&self.client);

        while let Some(subscription) = subscriptions.try_next().await? {
            if subscription.status == SubscriptionStatus::Active {
                return Ok(subscription.id.to_string());
            }
        }

        Err(PaymentError::Generic(String::from(
            "no active subscription found",
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
