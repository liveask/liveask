use mailjet_rs::common::Recipient;
use mailjet_rs::v3::Message;
use mailjet_rs::{Client, SendAPIVersion};
use mailjet_rs::{Map, Value};

pub struct MailJetCredentials {
    pub public_key: String,
    pub private_key: String,
}

#[allow(dead_code)]
async fn send_mail(
    receiver: String,
    event_name: String,
    public_link: String,
    mod_link: String,
    mailjet_template_id: usize, //std::env::var("MAILJET_TEMPLATE_ID").unwrap().parse::<usize>().unwrap()
    //TODO:
    //std::env::var("MAILJET_KEY").unwrap()
    //std::env::var("MAILJET_SECRET").unwrap()
    mailjet_credentials: MailJetCredentials,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new(
        SendAPIVersion::V3,
        &mailjet_credentials.public_key,
        &mailjet_credentials.private_key,
    );

    // Create your a `Message` instance with the minimum required values
    let mut message = Message::new(
        "mail@live-ask.com",
        "liveask",
        Some("New Event Created".to_string()),
        None,
    );
    message.push_recipient(Recipient::new(&receiver));

    message.set_template_id(mailjet_template_id);

    let mut vars = Map::new();

    vars.insert(String::from("eventname"), Value::from(event_name));
    vars.insert(String::from("publiclink"), Value::from(public_link));
    vars.insert(String::from("moderatorlink"), Value::from(mod_link));

    message.vars = Some(vars);

    let response = client.send(message).await;

    tracing::debug!("mailjet response: {:?}", response);

    Ok(())
}
