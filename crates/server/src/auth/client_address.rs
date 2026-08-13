//! Trusted client-network attribution for anonymous abuse controls.

use std::net::{IpAddr, SocketAddr};

use axum::http::HeaderMap;
use ipnet::IpNet;

const FORWARDED_FOR_HEADER: &str = "x-forwarded-for";
const MAX_FORWARDED_ADDRESSES: usize = 8;
const UNKNOWN_CLIENT_NETWORK: &[u8] = b"unknown-client-network";

/// Resolves one rate-limit identity from the transport peer and an explicitly
/// trusted reverse-proxy chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAddressPolicy {
    trusted_proxies: Vec<IpNet>,
}

impl ClientAddressPolicy {
    /// Uses only the TCP peer. Request headers receive no identity authority.
    pub fn direct() -> Self {
        Self {
            trusted_proxies: Vec::new(),
        }
    }

    /// Parses a nonempty comma-separated list of trusted proxy CIDRs.
    pub fn behind_trusted_proxies(value: &str) -> Result<Self, String> {
        let trusted_proxies = value
            .split(',')
            .map(str::trim)
            .map(|value| {
                value
                    .parse::<IpNet>()
                    .map_err(|_| "PLE_TRUSTED_PROXY_CIDRS contains an invalid CIDR".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if trusted_proxies.is_empty() || trusted_proxies.len() > MAX_FORWARDED_ADDRESSES {
            return Err("PLE_TRUSTED_PROXY_CIDRS must contain 1 to 8 CIDRs".to_string());
        }
        Ok(Self { trusted_proxies })
    }

    /// Returns a coarse network-prefix identity suitable only for a keyed
    /// rate-limit digest. IPv4 uses /24 and IPv6 uses /56: enough aggregation
    /// to bound a botnet's single network while avoiding a per-device quota at
    /// a campus NAT. Malformed trusted-proxy input shares one fail-closed
    /// bucket.
    pub(crate) fn rate_limit_identity(&self, peer: SocketAddr, headers: &HeaderMap) -> Vec<u8> {
        let peer_ip = canonical_ip(peer.ip());
        if !self.is_trusted(peer_ip) {
            return coarse_network_identity(peer_ip);
        }
        let Some(forwarded) = forwarded_addresses(headers) else {
            return UNKNOWN_CLIENT_NETWORK.to_vec();
        };
        forwarded
            .into_iter()
            .rev()
            .map(canonical_ip)
            .find(|address| !self.is_trusted(*address))
            .map_or_else(|| UNKNOWN_CLIENT_NETWORK.to_vec(), coarse_network_identity)
    }

    fn is_trusted(&self, address: IpAddr) -> bool {
        self.trusted_proxies
            .iter()
            .any(|network| network.contains(&address))
    }
}

fn coarse_network_identity(address: IpAddr) -> Vec<u8> {
    match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2]).into_bytes()
        }
        IpAddr::V6(address) => {
            let mut octets = address.octets();
            octets[7..].fill(0);
            format!("{}/56", std::net::Ipv6Addr::from(octets)).into_bytes()
        }
    }
}

fn forwarded_addresses(headers: &HeaderMap) -> Option<Vec<IpAddr>> {
    // A trusted proxy must send one canonical X-Forwarded-For chain. Multiple
    // field lines have proxy-specific merge behavior, so accepting them could
    // make the API and the proxy disagree about which address is the client.
    let mut values = headers.get_all(FORWARDED_FOR_HEADER).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }

    let mut addresses = Vec::new();
    for part in value.split(',') {
        if addresses.len() == MAX_FORWARDED_ADDRESSES {
            return None;
        }
        addresses.push(part.trim().parse().ok()?);
    }
    (!addresses.is_empty()).then_some(addresses)
}

fn canonical_ip(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(value) => value.to_ipv4_mapped().map_or(IpAddr::V6(value), IpAddr::V4),
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(value: &str) -> SocketAddr {
        SocketAddr::new(value.parse().expect("test IP address"), 443)
    }

    fn forwarded_headers(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(
                FORWARDED_FOR_HEADER,
                value.parse().expect("test header value"),
            );
        }
        headers
    }

    #[test]
    fn direct_policy_ignores_spoofed_forwarding_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(FORWARDED_FOR_HEADER, "198.51.100.9".parse().unwrap());
        assert_eq!(
            ClientAddressPolicy::direct().rate_limit_identity(peer("192.0.2.4"), &headers),
            b"192.0.2.0/24"
        );
    }

    #[test]
    fn trusted_policy_walks_from_peer_to_first_untrusted_address() {
        let policy =
            ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8, 2001:db8:100::/48").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            FORWARDED_FOR_HEADER,
            "203.0.113.99, 192.0.2.44, 10.1.2.3".parse().unwrap(),
        );
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &headers),
            b"192.0.2.0/24"
        );
    }

    #[test]
    fn trusted_policy_rejects_attacker_prepended_addresses() {
        let policy = ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8").unwrap();
        let headers = forwarded_headers(&["203.0.113.81, 198.51.100.44, 10.4.5.6"]);

        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &headers),
            b"198.51.100.0/24"
        );
    }

    #[test]
    fn trusted_policy_handles_ipv6_and_canonicalizes_ipv4_mapped_addresses() {
        let policy =
            ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8, 2001:db8:100::/48").unwrap();

        let ipv6_headers = forwarded_headers(&["2001:db8:ffff::44, 2001:db8:100::9"]);
        assert_eq!(
            policy.rate_limit_identity(peer("2001:db8:100::8"), &ipv6_headers),
            b"2001:db8:ffff::/56"
        );

        let mapped_headers = forwarded_headers(&["::ffff:192.0.2.44, ::ffff:10.4.5.6"]);
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &mapped_headers),
            b"192.0.2.0/24"
        );
    }

    #[test]
    fn trusted_policy_rejects_ambiguous_or_oversized_forwarding_chains() {
        let policy = ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8").unwrap();
        let repeated_lines = forwarded_headers(&["198.51.100.44", "10.4.5.6"]);
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &repeated_lines),
            UNKNOWN_CLIENT_NETWORK
        );

        let oversized =
            forwarded_headers(&["198.51.100.1, 198.51.100.2, 198.51.100.3, 198.51.100.4, \
             198.51.100.5, 198.51.100.6, 198.51.100.7, 198.51.100.8, 198.51.100.9"]);
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &oversized),
            UNKNOWN_CLIENT_NETWORK
        );
    }

    #[test]
    fn trusted_policy_rejects_all_trusted_or_malformed_chains() {
        let policy = ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8").unwrap();
        let all_trusted = forwarded_headers(&["10.1.2.3, 10.4.5.6"]);
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &all_trusted),
            UNKNOWN_CLIENT_NETWORK
        );

        let malformed = forwarded_headers(&["198.51.100.44, not-an-address"]);
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &malformed),
            UNKNOWN_CLIENT_NETWORK
        );
    }

    #[test]
    fn untrusted_peer_cannot_delegate_identity_with_a_convincing_chain() {
        let policy = ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8").unwrap();
        let headers = forwarded_headers(&["198.51.100.44, 10.4.5.6"]);

        assert_eq!(
            policy.rate_limit_identity(peer("192.0.2.99"), &headers),
            b"192.0.2.0/24"
        );
    }

    #[test]
    fn malformed_trusted_proxy_chain_uses_shared_unknown_bucket() {
        let policy = ClientAddressPolicy::behind_trusted_proxies("10.0.0.0/8").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(FORWARDED_FOR_HEADER, "not-an-address".parse().unwrap());
        assert_eq!(
            policy.rate_limit_identity(peer("10.9.8.7"), &headers),
            UNKNOWN_CLIENT_NETWORK
        );
    }

    #[test]
    fn ipv6_network_identity_is_a_coarse_prefix() {
        assert_eq!(
            ClientAddressPolicy::direct()
                .rate_limit_identity(peer("2001:db8:abcd:1234::9"), &HeaderMap::new()),
            b"2001:db8:abcd:1200::/56"
        );
    }
}
