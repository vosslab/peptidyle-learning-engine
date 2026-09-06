//! Idle worker lifecycle before typed Job execution is introduced.
//!
//! The process owns no HTTP listener and receives only its distinct database
//! Service Identity. M3 replaces this idle loop with named claim-and-commit
//! operations; it must not add direct protected-table access here.

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;

const WORKER_READINESS_BIND_ADDRESS: &str = "0.0.0.0:3001";

/// Wait for process termination after the worker login has been attested.
// ASVS 8.3.1 and 15.3.1: a worker starts with only a verified, least-privilege
// capability and acknowledges termination before its container grace period.
pub async fn run_until_shutdown() -> Result<()> {
    let listener = tokio::net::TcpListener::bind(WORKER_READINESS_BIND_ADDRESS)
        .await
        .context("could not bind the private worker readiness socket")?;
    tracing::info!(
        event = "worker_ready",
        readiness_address = WORKER_READINESS_BIND_ADDRESS
    );
    tokio::select! {
        () = shutdown_signal() => {
            tracing::info!(event = "worker_shutdown_requested");
            Ok(())
        }
        result = serve_readiness(listener) => result,
    }
}

async fn serve_readiness(listener: tokio::net::TcpListener) -> Result<()> {
    loop {
        let (mut stream, _peer) = listener
            .accept()
            .await
            .context("private worker readiness socket could not accept")?;
        // This is not an HTTP or job protocol. It accepts no request bytes and
        // returns one fixed liveness marker on the internal-only default network.
        stream
            .write_all(b"PLE-WORKER-READY\n")
            .await
            .context("private worker readiness socket could not respond")?;
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate = match tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGTERM", error = %error);
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(event = "worker_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
    }
}
