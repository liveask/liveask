use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrackingError {
    #[error("PH Async Error: {0}")]
    PosthogAsync(#[from] async_posthog::Error),
    #[error("PH Core Error: {0}")]
    PosthogCore(#[from] posthog_core::error::Error),
}

pub type TrackingResult<T> = std::result::Result<T, TrackingError>;
