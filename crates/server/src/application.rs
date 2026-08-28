//! Process mode parsing, router selection, and production server startup.
//!
//! Composition stays in the `server_core::composition` library module; this
//! file handles binding and the self-probe used by the container image.

use std::future::IntoFuture;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessMode {
    Api,
    HealthProbe,
    Worker,
    InvitationDeliveryWorker,
    PublicAssetPublisher,
    #[cfg(feature = "e2e-grader-fault")]
    DeterministicGraderExceptionWorker,
}

#[cfg(feature = "e2e-grader-fault")]
const PROCESS_USAGE: &str = concat!(
    "peptidyle-api [--health-probe|--worker|--invitation-delivery-worker|",
    "--public-asset-publisher|--deterministic-grader-exception-worker]",
);
#[cfg(not(feature = "e2e-grader-fault"))]
const PROCESS_USAGE: &str =
    "peptidyle-api [--health-probe|--worker|--invitation-delivery-worker|--public-asset-publisher]";

fn process_mode(arguments: &[String]) -> anyhow::Result<ProcessMode> {
    match arguments {
        [] => Ok(ProcessMode::Api),
        [flag] if flag == "--health-probe" => Ok(ProcessMode::HealthProbe),
        [flag] if flag == "--worker" => Ok(ProcessMode::Worker),
        [flag] if flag == "--invitation-delivery-worker" => {
            Ok(ProcessMode::InvitationDeliveryWorker)
        }
        [flag] if flag == "--public-asset-publisher" => Ok(ProcessMode::PublicAssetPublisher),
        #[cfg(feature = "e2e-grader-fault")]
        [flag] if flag == "--deterministic-grader-exception-worker" => {
            Ok(ProcessMode::DeterministicGraderExceptionWorker)
        }
        _ => anyhow::bail!("usage: {PROCESS_USAGE}"),
    }
}

pub(crate) async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .with_ansi(false)
        .try_init()
        .map_err(|error| anyhow::anyhow!("server logging could not be initialized: {error}"))?;
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mode = process_mode(&arguments)?;

    if mode == ProcessMode::Worker {
        return server_core::composition::run_production_worker_from_env().await;
    }
    if mode == ProcessMode::InvitationDeliveryWorker {
        return server_core::composition::run_production_invitation_delivery_worker_from_env()
            .await;
    }
    if mode == ProcessMode::PublicAssetPublisher {
        return server_core::composition::run_public_asset_publisher_from_env().await;
    }
    #[cfg(feature = "e2e-grader-fault")]
    if mode == ProcessMode::DeterministicGraderExceptionWorker {
        return server_core::composition::run_deterministic_grader_exception_worker_from_env()
            .await;
    }
    // Container health check mode. The same binary probes its own /health so
    // the runtime image needs no curl or wget, which keeps the attack surface
    // to one executable.
    if mode == ProcessMode::HealthProbe {
        let bind_addr = server_core::composition::bind_address_from_env()?;
        // The server binds 0.0.0.0 (every interface), which is not an address
        // a client can connect *to*. Probe the loopback interface on the same
        // port instead.
        let probe_addr = if bind_addr.ip().is_unspecified() {
            SocketAddr::from(([127, 0, 0, 1], bind_addr.port()))
        } else {
            bind_addr
        };
        return match server_core::health::probe_over_http(probe_addr) {
            Ok(()) => Ok(()),
            Err(message) => {
                eprintln!("health probe failed: {message}");
                std::process::exit(1);
            }
        };
    }

    let bind_addr = server_core::composition::bind_address_from_env()?;
    let app = server_core::composition::production_router_from_env().await?;

    let app = server_core::request_lifecycle::apply_request_lifecycle(app);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!(event = "api_listening", address = %bind_addr);
    run_api_until_shutdown(listener, app).await?;

    Ok(())
}

async fn run_api_until_shutdown(
    listener: tokio::net::TcpListener,
    app: axum::Router,
) -> anyhow::Result<()> {
    let (drain_signal, drain_requested) = tokio::sync::oneshot::channel::<()>();
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = drain_requested.await;
    })
    .into_future();
    tokio::pin!(server);
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    tokio::select! {
        result = &mut server => result.map_err(Into::into),
        () = &mut shutdown => {
            tracing::info!(event = "api_shutdown_requested");
            // A receiver can only be absent if the server ended first, which
            // this select arm excludes. Do not panic while stopping.
            let _ = drain_signal.send(());
            match tokio::time::timeout(
                server_core::request_lifecycle::API_DRAIN_TIMEOUT,
                &mut server,
            ).await {
                Ok(result) => result.map_err(Into::into),
                Err(_) => {
                    tracing::error!(
                        event = "api_shutdown_drain_expired",
                        drain_timeout_seconds = server_core::request_lifecycle::API_DRAIN_TIMEOUT.as_secs()
                    );
                    Ok(())
                }
            }
        }
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
                tracing::error!(event = "api_shutdown_signal_unavailable", signal = "SIGTERM", error = %error);
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    tracing::error!(event = "api_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(event = "api_shutdown_signal_unavailable", signal = "SIGINT", error = %error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_mode_is_an_exact_closed_command() {
        assert_eq!(process_mode(&[]).expect("API"), ProcessMode::Api);
        assert_eq!(
            process_mode(&["--health-probe".to_string()]).expect("probe"),
            ProcessMode::HealthProbe
        );
        assert_eq!(
            process_mode(&["--worker".to_string()]).expect("worker"),
            ProcessMode::Worker
        );
        assert_eq!(
            process_mode(&["--invitation-delivery-worker".to_string()])
                .expect("invitation delivery worker"),
            ProcessMode::InvitationDeliveryWorker
        );
        assert_eq!(
            process_mode(&["--public-asset-publisher".to_string()]).expect("publisher"),
            ProcessMode::PublicAssetPublisher
        );
        for invalid in [
            vec!["--unknown".to_string()],
            vec!["--local-worker".to_string()],
            vec!["--local-invitation-delivery-worker".to_string()],
            vec!["--worker".to_string(), "--health-probe".to_string()],
        ] {
            assert!(process_mode(&invalid).is_err());
        }
    }

    #[test]
    fn deterministic_grader_exception_mode_is_feature_gated() {
        let arguments = ["--deterministic-grader-exception-worker".to_string()];
        #[cfg(feature = "e2e-grader-fault")]
        assert_eq!(
            process_mode(&arguments).expect("feature mode"),
            ProcessMode::DeterministicGraderExceptionWorker
        );
        #[cfg(not(feature = "e2e-grader-fault"))]
        assert!(process_mode(&arguments).is_err());
    }
}
