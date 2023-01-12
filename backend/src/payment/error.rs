use paypal_rust::client::PayPalError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("General Error: {0}")]
    General(String),
    #[error("Paypal Error: {0}")]
    Paypal(#[from] PayPalError),
    #[error("Paypal Error: {0}")]
    PaypalBox(#[from] Box<PayPalError>),
}

pub type PaymentResult<T> = std::result::Result<T, PaymentError>;
