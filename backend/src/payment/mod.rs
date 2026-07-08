mod error;

use futures_util::TryStreamExt;
use std::str::FromStr;
use stripe::{
    BillingPortalSession, CheckoutSession, CheckoutSessionId, CheckoutSessionMode,
    CheckoutSessionStatus, Client, CreateBillingPortalSession, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionPaymentIntentData, Customer, CustomerId,
    ListCustomers, ListProducts, ListSubscriptions, Subscription, SubscriptionStatus,
};
use tracing::instrument;

pub use self::error::PaymentError;
use self::error::PaymentResult;

#[derive(Clone)]
pub struct Payment {
    client: Client,
    price: Option<String>,
    subscription_url: Option<String>,
    portal_login_url: Option<String>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
impl Default for Payment {
    fn default() -> Self {
        Self {
            client: Client::new(String::new()),
            price: None,
            subscription_url: None,
            portal_login_url: None,
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
            portal_login_url: None,
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

        match self.portal_login_url().await {
            Ok(Some(url)) => {
                self.portal_login_url = Some(url.clone());
                tracing::info!("[stripe] portal login url: {}", url);
            }
            Ok(None) => {
                tracing::warn!("[stripe] no portal login page configured");
            }
            Err(e) => {
                tracing::error!("[stripe] failed to fetch portal login url: {:?}", e);
            }
        }

        premium_product
            .ok_or_else(|| PaymentError::Generic(String::from("no premium product found")))
    }

    #[instrument(skip(self))]
    async fn fetch_payment_link_url(&self) -> PaymentResult<Option<String>> {
        // Using raw HTTP because async-stripe 0.31 fails to deserialize
        // `"type": "self"` in payment link subscription_data.invoice_settings.issuer
        use serde_json::Value;

        #[derive(serde::Serialize)]
        struct Params {
            active: bool,
        }
        let response = self
            .client
            .get_query("/payment_links", &Params { active: true })
            .await?;

        if let Value::Object(obj) = response {
            if let Some(Value::Array(data)) = obj.get("data") {
                tracing::info!("[stripe] found {} active payment links", data.len());

                for item in data {
                    if let Some(Value::String(url)) = item.get("url") {
                        return Ok(Some(url.clone()));
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

    pub fn portal_login_url_cached(&self) -> Option<&str> {
        self.portal_login_url.as_deref()
    }

    #[instrument(skip(self))]
    pub async fn portal_login_url(&self) -> PaymentResult<Option<String>> {
        use serde_json::Value;

        let response = self.client.get("/billing_portal/configurations").await?;

        if let Value::Object(obj) = response {
            if let Some(Value::Array(data)) = obj.get("data") {
                for item in data {
                    if let Value::Object(config) = item {
                        if let Some(Value::Object(login_page)) = config.get("login_page") {
                            if matches!(login_page.get("enabled"), Some(Value::Bool(true))) {
                                if let Some(Value::String(url)) = login_page.get("url") {
                                    return Ok(Some(url.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub async fn customer_portal_url(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> PaymentResult<String> {
        let customer = CustomerId::from_str(customer_id)?;
        let mut params = CreateBillingPortalSession::new(customer);
        params.return_url = Some(return_url);

        let session = BillingPortalSession::create(&self.client, params).await?;
        Ok(session.url)
    }

    fn checkout_email(sess: &CheckoutSession) -> Option<&str> {
        sess.customer_details
            .as_ref()
            .and_then(|details| details.email.as_deref())
            .or(sess.customer_email.as_deref())
    }

    #[instrument(skip(self, email))]
    async fn active_customer_by_email(&self, email: &str) -> PaymentResult<Option<String>> {
        let params = ListCustomers {
            email: Some(email),
            limit: Some(100),
            ..Default::default()
        };

        for customer in Customer::list(&self.client, &params).await?.data {
            let customer_id = customer.id.to_string();

            if self.verify_customer(customer_id.as_str()).await.is_ok() {
                return Ok(Some(customer_id));
            }
        }

        Ok(None)
    }

    #[instrument(skip(self, email))]
    pub async fn subscription_customer_by_email(&self, email: &str) -> PaymentResult<String> {
        self.active_customer_by_email(email).await?.ok_or_else(|| {
            PaymentError::Generic(String::from(
                "no active subscription customer found for email",
            ))
        })
    }

    #[instrument(skip(self))]
    pub async fn subscription_checkout(&self, checkout: String) -> PaymentResult<String> {
        let sess = CheckoutSessionId::from_str(checkout.as_str())?;

        let sess = CheckoutSession::retrieve(&self.client, &sess, &[]).await?;

        tracing::info!(
            has_customer = %sess.customer.is_some(),
            has_subscription = %sess.subscription.is_some(),
            has_checkout_email = %Self::checkout_email(&sess).is_some(),
            status = ?sess.status,
            payment_status = ?sess.payment_status,
            "subscription checkout session retrieved"
        );

        if let Some(customer) = &sess.customer {
            return Ok(customer.id().to_string());
        }

        if let Some(subscription) = &sess.subscription {
            let subscription_id = subscription.id();
            let subscription = Subscription::retrieve(&self.client, &subscription_id, &[]).await?;

            return Ok(subscription.customer.id().to_string());
        }

        if let Some(email) = Self::checkout_email(&sess) {
            tracing::info!(
                "checkout session missing customer; looking up existing customer by email"
            );

            if let Some(customer_id) = self.active_customer_by_email(email).await? {
                return Ok(customer_id);
            }
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
            params.payment_intent_data = Some(CreateCheckoutSessionPaymentIntentData {
                description: Some(format!("Premium Event: {}", mod_url)),
                metadata: Some(
                    vec![
                        (String::from("event"), event.to_string()),
                        (String::from("mod"), mod_url.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                ),
                ..Default::default()
            });
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

#[cfg(test)]
mod tests {
    use super::Payment;
    use stripe::{CheckoutSession, PaymentPagesCheckoutSessionCustomerDetails};

    #[test]
    fn checkout_email_prefers_customer_details() {
        let session = CheckoutSession {
            customer_details: Some(PaymentPagesCheckoutSessionCustomerDetails {
                email: Some(String::from("details@example.com")),
                ..Default::default()
            }),
            customer_email: Some(String::from("fallback@example.com")),
            ..Default::default()
        };

        assert_eq!(
            Payment::checkout_email(&session),
            Some("details@example.com")
        );
    }

    #[test]
    fn checkout_email_falls_back_to_customer_email() {
        let session = CheckoutSession {
            customer_email: Some(String::from("fallback@example.com")),
            ..Default::default()
        };

        assert_eq!(
            Payment::checkout_email(&session),
            Some("fallback@example.com")
        );
    }
}
