use axum::response::{IntoResponse, Response};
use deadpool_redis::{CreatePoolError, PoolError};
use redis::RedisError;
use reqwest::StatusCode;
use thiserror::Error;

use crate::{eventsdb, payment::PaymentError};

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("General Error: {0}")]
    General(String),

    #[error("Acceesssing Deleted Event: {0}")]
    AccessingDeletedEvent(String),

    #[error("Trying to modify timed out Event: {0}")]
    ModifyingTimedOutEvent(String),

    #[error("Events DB Error: {0}")]
    EventsDB(#[from] eventsdb::Error),

    #[error("Serde Json Error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Payment Error: {0}")]
    Payment(#[from] PaymentError),

    #[error("DeadPool Create Error: {0}")]
    DeadPoolCreatePool(#[from] CreatePoolError),

    #[error("DeadPool Redis Error: {0}")]
    DeadPoolRedis(#[from] PoolError),

    #[error("Redis Error: {0}")]
    Redis(#[from] RedisError),

    #[error("Uri Error: {0}")]
    Uri(#[from] axum::http::uri::InvalidUri),
}

impl IntoResponse for InternalError {
    #[allow(clippy::cognitive_complexity)]
    fn into_response(self) -> Response {
        match self {
            Self::General(e) => {
                tracing::error!("{e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
            }

            Self::AccessingDeletedEvent(id) => {
                tracing::info!("accessing deleted event: {id}");
                (StatusCode::BAD_REQUEST, "").into_response()
            }

            Self::ModifyingTimedOutEvent(id) => {
                tracing::info!("trying to modify timed out Event: {id}");
                (StatusCode::BAD_REQUEST, "").into_response()
            }

            Self::Payment(e) => {
                tracing::error!("payment error: {e}");
                (StatusCode::BAD_REQUEST, "").into_response()
            }

            Self::SerdeJson(e) => {
                tracing::error!("serde error: {e}");
                (StatusCode::BAD_REQUEST, "").into_response()
            }

            Self::EventsDB(e) if matches!(e, eventsdb::Error::Concurrency) => {
                tracing::info!("concurrency collision: {e}");

                (
                    StatusCode::CONFLICT,
                    String::from("DB: Conditional write failed"),
                )
                    .into_response()
            }

            //Note: do not trace this as error
            Self::EventsDB(e) if matches!(e, eventsdb::Error::ItemNotFound) => {
                tracing::info!("ItemNotFound error: {}", e);
                (StatusCode::BAD_REQUEST, "").into_response()
            }

            Self::EventsDB(e) => convert_error(e),
            Self::Redis(e) => convert_error(e),
            Self::Uri(e) => convert_error(e),
            Self::DeadPoolCreatePool(e) => convert_error(e),
            Self::DeadPoolRedis(e) => convert_error(e),
        }
    }
}

pub type Result<T> = std::result::Result<T, InternalError>;

fn convert_error<E: std::error::Error>(e: E) -> Response {
    tracing::error!("convert_error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
}

#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::error::InternalError::General(format!($msg)))
    };
    ($err:expr $(,)?) => {
        return Err($crate::error::InternalError::General(format!($err)))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::error::InternalError::General(format!($fmt, $($arg)*)))
    };
}
