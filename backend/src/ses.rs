use aws_sdk_ses::types::{Body, Content, Destination, Message};

pub async fn send_message(
    client: &aws_sdk_ses::Client,
    to: &str,
    subject: &str,
    message: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dest: Destination = Destination::builder().to_addresses(to).build();

    let subject_content = Content::builder().data(subject).charset("UTF-8").build();
    let body_content = Content::builder().data(message).charset("UTF-8").build();
    let body = Body::builder().text(body_content).build();

    let msg = Message::builder()
        .subject(subject_content)
        .body(body)
        .build();

    client
        .send_email()
        .destination(dest)
        .message(msg)
        .source("mail@live-ask.com")
        .send()
        .await?;

    tracing::info!("email sent");

    Ok(())
}
