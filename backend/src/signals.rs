#[cfg(target_family = "unix")]
use tokio::signal::unix::{signal, SignalKind};

#[cfg(target_family = "unix")]
pub fn create_term_signal_handler(sender: tokio::sync::oneshot::Sender<()>) {
    tokio::spawn(async move {
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                tracing::info!("register terminate signal handler");

                stream.recv().await;

                tracing::info!("got terminate signal");
            }
            Err(e) => {
                tracing::error!("signal error: {e}");
            }
        }

        let _: Result<(), _> = sender.send(());
    });
}

#[allow(clippy::needless_pass_by_value)]
#[cfg(not(target_family = "unix"))]
pub fn create_term_signal_handler(_sender: tokio::sync::oneshot::Sender<()>) {}
