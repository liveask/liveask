mod error;

pub use self::error::PaymentError;
use self::error::PaymentResult;
use paypal_rust::{
    client::AppInfo, AmountWithBreakdown, Client, CreateOrderDto, Environment, Order,
    OrderApplicationContext, OrderIntent, PurchaseUnitRequest,
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
}

impl Default for Payment {
    fn default() -> Self {
        Self {
            client: Client::new(String::new(), String::new(), Environment::Sandbox),
        }
    }
}

//TODO: fix all `expect`s
impl Payment {
    pub async fn new(username: String, password: String, sandbox: bool) -> PaymentResult<Self> {
        let client = Client::new(
            username,
            password,
            if sandbox {
                Environment::Sandbox
            } else {
                Environment::Live
            },
        )
        .with_app_info(AppInfo {
            name: "liveask".to_string(),
            version: "1.0".to_string(),
            website: None,
        });

        client.authenticate().await?;

        Ok(Self { client })
    }

    pub async fn create_order(&self, event: String, return_url: String) -> PaymentResult<String> {
        let order = Order::create(
            &self.client,
            CreateOrderDto {
                intent: OrderIntent::Capture,
                purchase_units: vec![PurchaseUnitRequest {
                    custom_id: Some(event.clone()),
                    amount: AmountWithBreakdown {
                        currency_code: String::from("EUR"),
                        value: String::from("5.99"),
                        breakdown: None,
                    },
                    ..Default::default()
                }],
                application_context: Some(OrderApplicationContext::new().return_url(return_url)),
                payer: None,
            },
        )
        .await?;

        tracing::info!("order: {:?}", order);

        Ok(order
            .links
            .expect("TODO")
            .iter()
            .find(|e| e.rel == "approve")
            .expect("TODO")
            .href
            .clone())
    }

    pub async fn capture_approved_payment(&self, id: String) -> PaymentResult<()> {
        let order = Order::show_details(&self.client, &id).await.expect("TODO");

        let unit = order
            .purchase_units
            .and_then(|units| units.first().cloned())
            .expect("TODO");

        let event_id = unit.custom_id.expect("TODO");

        tracing::info!(
            "order: {} - {:?} - {}",
            order.id.unwrap_or_default(),
            order.status,
            event_id
        );

        let authorized_payment = Order::capture(&self.client, &id, None)
            .await
            //TODO:
            .expect("TODO");

        tracing::info!("auth: {:?}", authorized_payment);

        Ok(())
    }
}
