use mailjet_rs::common::Recipient;
use mailjet_rs::v3::Message;
use mailjet_rs::{Client, SendAPIVersion};
use mailjet_rs::{Map, Value};

use crate::env;

#[derive(Clone, Debug)]
pub struct MailJetCredentials {
    pub public_key: String,
    pub private_key: String,
}

#[derive(Clone, Debug)]
pub struct MailjetConfig {
    pub credentials: MailJetCredentials,
    pub template_id: usize,
}

impl MailjetConfig {
    pub fn new() -> Option<Self> {
        let template_id = std::env::var(env::ENV_MAILJET_TEMPLATE_ID)
            .ok()?
            .parse::<usize>()
            .ok()?;

        let key = std::env::var(env::ENV_MAILJET_KEY).ok()?;
        let secret = std::env::var(env::ENV_MAILJET_SECRET).ok()?;

        Some(Self {
            template_id,
            credentials: MailJetCredentials {
                public_key: key,
                private_key: secret,
            },
        })
    }

    pub async fn send_mail(
        &self,
        receiver: String,
        event_name: String,
        public_link: String,
        mod_link: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new(
            SendAPIVersion::V3,
            &self.credentials.public_key,
            &self.credentials.private_key,
        );

        // Create your a `Message` instance with the minimum required values
        let mut message = Message::new(
            "mail@live-ask.com",
            "liveask",
            Some("New Event Created".to_string()),
            None,
        );
        message.push_recipient(Recipient::new(&receiver));

        message.set_template_id(self.template_id);

        let mut vars = Map::new();

        vars.insert(String::from("eventname"), Value::from(event_name));
        vars.insert(String::from("publiclink"), Value::from(public_link));
        vars.insert(String::from("moderatorlink"), Value::from(mod_link));

        message.vars = Some(vars);

        let response = client.send(message).await;

        tracing::debug!("mailjet response: {:?}", response);

        Ok(())
    }
}
