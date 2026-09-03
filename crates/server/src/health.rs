//! Readiness reporting for the container health check.
//!
//! The rule the containers are gated on: `/health` reports ready only when
//! every backing dependency answered a real request. A health endpoint that
//! returns 200 because the process is alive tells an orchestrator nothing, so
//! readiness here is a function of probe results and has no default.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// The result of probing one backing dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    /// Dependency name as it appears in the health body, for example
    /// `postgres` or `object-store`.
    pub name: &'static str,
    /// Whether the dependency answered a real request.
    pub ok: bool,
}

impl ProbeResult {
    /// Records a dependency that answered.
    pub fn ready(name: &'static str) -> Self {
        ProbeResult { name, ok: true }
    }

    /// Records a dependency that did not answer.
    pub fn failed(name: &'static str) -> Self {
        ProbeResult { name, ok: false }
    }
}

/// Whether the process is ready to serve traffic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    /// Every probe answered.
    Ready,
    /// At least one probe failed; carries the failing dependency names.
    Degraded(Vec<&'static str>),
}

/// Decides readiness from the probe results.
///
/// An empty probe list is `Degraded`, not `Ready`. A process that has not
/// checked anything has not proven anything, and treating "no evidence" as
/// health defeats meaningful container readiness reporting.
///
/// # Examples
///
/// ```
/// use server_core::health::{readiness, ProbeResult, Readiness};
///
/// let probes = vec![ProbeResult::ready("postgres"), ProbeResult::ready("object-store")];
/// assert_eq!(readiness(&probes), Readiness::Ready);
///
/// let probes = vec![ProbeResult::ready("postgres"), ProbeResult::failed("object-store")];
/// assert_eq!(readiness(&probes), Readiness::Degraded(vec!["object-store"]));
/// ```
pub fn readiness(probes: &[ProbeResult]) -> Readiness {
    if probes.is_empty() {
        return Readiness::Degraded(vec!["no-probes-configured"]);
    }
    let failed: Vec<&'static str> = probes
        .iter()
        .filter(|probe| !probe.ok)
        .map(|probe| probe.name)
        .collect();
    if failed.is_empty() {
        Readiness::Ready
    } else {
        Readiness::Degraded(failed)
    }
}

/// Asks a running server for its own readiness over HTTP.
///
/// This backs the container `HEALTHCHECK`. It speaks HTTP/1.1 directly over a
/// socket rather than pulling in an HTTP client, which keeps the runtime image
/// down to one executable with no curl or wget to be borrowed by an attacker.
///
/// Only the status line matters: 200 means every dependency probe answered.
///
/// # Errors
///
/// Returns a message when the socket cannot be opened, the exchange times out,
/// or the response is anything other than 200.
pub fn probe_over_http(addr: SocketAddr) -> Result<(), String> {
    // A health check that can hang is worse than one that fails: the
    // orchestrator would wait instead of restarting.
    let timeout = Duration::from_secs(2);

    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|error| format!("connect: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("write timeout: {error}"))?;

    let request = format!("GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("read: {error}"))?;

    let status_line = response.lines().next().unwrap_or_default();
    if status_line.contains(" 200 ") {
        Ok(())
    } else {
        Err(format!("unexpected status: {status_line}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_probes_answering_is_ready() {
        let probes = vec![
            ProbeResult::ready("postgres"),
            ProbeResult::ready("object-store"),
        ];
        assert_eq!(readiness(&probes), Readiness::Ready);
    }

    #[test]
    fn one_failing_probe_names_that_dependency() {
        let probes = vec![
            ProbeResult::failed("postgres"),
            ProbeResult::ready("object-store"),
        ];
        assert_eq!(readiness(&probes), Readiness::Degraded(vec!["postgres"]));
    }

    #[test]
    fn no_probes_is_not_ready() {
        assert_eq!(
            readiness(&[]),
            Readiness::Degraded(vec!["no-probes-configured"])
        );
    }
}
