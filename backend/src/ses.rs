use aws_sdk_ses::{
    operation::send_email::SendEmailOutput,
    types::{Body, Content, Destination, Message},
};

pub async fn send_message(
    client: &aws_sdk_ses::Client,
    to: &[String],
    subject: &str,
    message: &str,
    sender: &str,
) -> std::result::Result<SendEmailOutput, Box<dyn std::error::Error + Send + Sync>> {
    let dest = Destination::builder()
        .set_to_addresses(Some(to.into()))
        .build();

    let subject_content = Content::builder().data(subject).charset("UTF-8").build()?;
    let body_content = Content::builder().data(message).charset("UTF-8").build()?;
    let body = Body::builder().html(body_content).build();

    let msg = Message::builder()
        .subject(subject_content)
        .body(body)
        .build();

    let output = client
        .send_email()
        .destination(dest)
        .message(msg)
        .source(sender)
        .send()
        .await?;

    Ok(output)
}
