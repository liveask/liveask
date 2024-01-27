use stripe::{ParseIdError, StripeError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("Stripe Error: {0}")]
    Paypal(#[from] StripeError),
    #[error("Stripe Id Error: {0}")]
    IdError(#[from] ParseIdError),
}

pub type PaymentResult<T> = std::result::Result<T, PaymentError>;
