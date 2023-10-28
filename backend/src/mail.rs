use std::collections::HashMap;

use handlebars::Handlebars;
use tracing::instrument;

use crate::{aws_ses_client, ses};

#[derive(Clone, Debug)]
pub struct MailConfig;

const MAIL_TEMPLATE: &str = include_str!("../mail_template.html.hbs");

impl MailConfig {
    pub const fn new() -> Self {
        Self {}
    }

    fn create_mail(
        event_name: &str,
        public_link: &str,
        mod_link: &str,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut hb = Handlebars::new();
        hb.register_template_string("template", MAIL_TEMPLATE)?;

        let mut data: HashMap<&str, &str> = HashMap::with_capacity(3);
        data.insert("event_name", event_name);
        data.insert("short_link", public_link);
        data.insert("mod_link", mod_link);
        let content = hb.render("template", &data)?;

        Ok(content)
    }

    #[instrument(err, skip(self, mod_link))]
    pub async fn send_mail(
        &self,
        event_id: String,
        receiver: String,
        event_name: String,
        public_link: String,
        mod_link: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("mail::send_mail: {event_id}");

        let content = Self::create_mail(&event_name, &public_link, &mod_link)?;

        let client = aws_ses_client().await?;
        let response = ses::send_message(
            &client,
            &[receiver],
            "New Event Created",
            &content,
            "mail@live-ask.com",
        )
        .await?;

        tracing::info!("mail sent response: {:?}", response);

        Ok(())
    }
}
