//! Signal handling for graceful shutdown

use signal_hook_tokio::Signals;
use futures::stream::StreamExt;
use tracing::info;

/// Wait for shutdown signals (SIGTERM, SIGINT)
pub async fn shutdown_signal() {
    let mut signals = Signals::new(&[
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
    ]).expect("Failed to create signal handler");

    while let Some(signal) = signals.next().await {
        info!("Received signal: {}", signal);
        break;
    }
}
