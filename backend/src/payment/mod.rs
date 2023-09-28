mod error;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub use self::error::PaymentError;
use self::error::PaymentResult;
use paypal_rust::{
    client::AppInfo, AmountWithBreakdown, Client, CreateOrderDto, Environment, Order,
    OrderApplicationContext, OrderIntent, OrderStatus, PurchaseUnitRequest,
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PaymentCheckoutApprovedResource {
    pub id: String,
    pub status: String,
    pub intent: String,
    pub create_time: String,
}

#[derive(Deserialize, Debug)]
pub struct PaymentCaptureRefundedResource {
    pub id: String,
    pub status: String,
    pub custom_id: Option<String>,
    pub note_to_payer: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PaymentWebhookBase {
    pub id: String,
    pub create_time: String,
    pub resource_type: String,
    pub event_type: String,
    pub summary: String,
    pub resource: serde_json::Value,
}

#[derive(Clone)]
pub struct Payment {
    client: Client,
    authenticated: Arc<AtomicBool>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
impl Default for Payment {
    fn default() -> Self {
        Self {
            client: Client::new(String::new(), String::new(), Environment::Sandbox).expect(""),
            authenticated: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Payment {
    pub fn new(username: String, password: String, sandbox: bool) -> PaymentResult<Self> {
        let client = Client::new(
            username,
            password,
            if sandbox {
                Environment::Sandbox
            } else {
                Environment::Live
            },
        )?
        .with_app_info(&AppInfo {
            name: "liveask".to_string(),
            version: crate::GIT_HASH.to_string(),
            website: Some("www.live-ask.com".to_string()),
        });

        Ok(Self {
            client,
            authenticated: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn authenticate(&self) -> PaymentResult<()> {
        if !self.authenticated.load(Ordering::Relaxed) {
            self.client.authenticate().await?;
            self.authenticated.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    pub async fn create_order(
        &self,
        event: &str,
        mod_url: &str,
        return_url: &str,
    ) -> PaymentResult<String> {
        self.authenticate().await?;

        let order = Order::create(
            &self.client,
            CreateOrderDto {
                intent: OrderIntent::Capture,
                purchase_units: vec![PurchaseUnitRequest {
                    description: Some(format!("live-ask premium: {mod_url}")),
                    custom_id: Some(event.to_string()),
                    amount: AmountWithBreakdown {
                        currency_code: String::from("EUR"),
                        value: String::from("7"),
                        breakdown: None,
                    },
                    ..Default::default()
                }],
                application_context: Some(
                    OrderApplicationContext::new().return_url(return_url.to_string()),
                ),
                payer: None,
            },
        )
        .await?;

        tracing::debug!("order: {:?}", order);

        Ok(order
            .links
            .ok_or_else(|| PaymentError::General(String::from("links not populated")))?
            .iter()
            .find(|e| e.rel == "approve")
            .ok_or_else(|| PaymentError::General(String::from("approve link not found")))?
            .href
            .clone())
    }

    pub async fn event_of_order(&self, order_id: String) -> PaymentResult<String> {
        self.authenticate().await?;

        let order = Order::show_details(&self.client, &order_id).await?;

        let unit = order
            .purchase_units
            .and_then(|units| {
                if units.len() > 1 {
                    tracing::warn!(
                        "payment contains more than expected PurchaseUnits: {}",
                        units.len()
                    );
                }
                units.first().cloned()
            })
            .ok_or_else(|| PaymentError::General(String::from("purchase unit not found")))?;

        let event_id = unit
            .custom_id
            .ok_or_else(|| PaymentError::General(String::from("custom id not found")))?;

        tracing::info!(
            "order: {} - {:?} - {}",
            order.id.unwrap_or_default(),
            order.status,
            event_id
        );

        Ok(event_id)
    }

    pub async fn capture_payment(&self, order_id: String) -> PaymentResult<bool> {
        self.authenticate().await?;

        let captured_ordered = Order::capture(&self.client, &order_id, None).await?;

        tracing::debug!("auth: {:?}", captured_ordered);

        let completed = captured_ordered
            .status
            .map(|status| status == OrderStatus::Completed)
            .unwrap_or_default();

        if !completed {
            tracing::warn!("paypment capture failed: {:?}", captured_ordered);
        }

        Ok(completed)
    }
}
