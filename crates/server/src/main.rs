//! MOD-SRV: thin production binary entry point.
//!
//! Composition stays in `composition.rs`; this file handles binding and the
//! self-probe used by the container image.

mod composition;

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind_addr = composition::bind_address_from_env()?;

    // Container health check mode. The same binary probes its own /health so
    // the runtime image needs no curl or wget, which keeps the attack surface
    // to one executable.
    if std::env::args().any(|arg| arg == "--health-probe") {
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

    let app = composition::production_router_from_env().await?;

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    eprintln!("peptidyle api listening on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
