//! MOD-SRV: thin production binary entry point.
//!
//! Composition stays in the `server_core::composition` library module; this
//! file handles binding and the self-probe used by the container image.

use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessMode {
    Api,
    HealthProbe,
    Worker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiRouterMode {
    Production,
    LocalDevelopment,
}

fn process_mode(arguments: &[String]) -> anyhow::Result<ProcessMode> {
    match arguments {
        [] => Ok(ProcessMode::Api),
        [flag] if flag == "--health-probe" => Ok(ProcessMode::HealthProbe),
        [flag] if flag == "--worker" => Ok(ProcessMode::Worker),
        _ => anyhow::bail!("usage: peptidyle-api [--health-probe|--worker]"),
    }
}

fn api_router_mode(local_development_auth: Option<&str>) -> anyhow::Result<ApiRouterMode> {
    match local_development_auth {
        None | Some("0") => Ok(ApiRouterMode::Production),
        Some("1") => Ok(ApiRouterMode::LocalDevelopment),
        Some(value) => anyhow::bail!(
            "PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH must be unset, 0, or exactly 1; got {value:?}"
        ),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    let app = match api_router_mode(
        std::env::var("PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH")
            .ok()
            .as_deref(),
    )? {
        ApiRouterMode::Production => server_core::composition::production_router_from_env().await?,
        ApiRouterMode::LocalDevelopment => {
            server_core::composition::local_development_router_from_env().await?
        }
    };

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    eprintln!("peptidyle api listening on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
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
        for invalid in [
            vec!["--unknown".to_string()],
            vec!["--worker".to_string(), "--health-probe".to_string()],
        ] {
            assert!(process_mode(&invalid).is_err());
        }
    }

    #[test]
    fn local_development_router_selection_is_exact_and_fail_closed() {
        assert_eq!(
            api_router_mode(None).expect("production default"),
            ApiRouterMode::Production
        );
        assert_eq!(
            api_router_mode(Some("0")).expect("production disabled"),
            ApiRouterMode::Production
        );
        assert_eq!(
            api_router_mode(Some("1")).expect("explicit local development"),
            ApiRouterMode::LocalDevelopment
        );
        for invalid in ["", "01", "true", "2"] {
            assert!(api_router_mode(Some(invalid)).is_err(), "{invalid:?}");
        }
    }
}
