use std::net::{IpAddr, SocketAddr};
use axum::http::HeaderMap;
use ipnet::IpNet;

/// Constant-time string comparison to prevent timing attacks
pub fn secure_compare(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut result = 0;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Normalize IP address by stripping IPv6 mapping prefix (::ffff:) if present
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => {
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                IpAddr::V4(ipv4)
            } else {
                IpAddr::V6(ipv6)
            }
        }
        IpAddr::V4(ipv4) => IpAddr::V4(ipv4),
    }
}

/// Parse a comma-separated list of trusted proxy IPs/CIDRs, ignoring inline shell-style comments
pub fn parse_trusted_proxies(raw: &str) -> Vec<IpNet> {
    raw.split(',')
        .map(|entry| {
            // Strip comments
            let without_comment = entry.split('#').next().unwrap_or("");
            without_comment.trim()
        })
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            // If it doesn't contain a '/' CIDR notation, parse it as a single IP and convert to a /32 or /128 net
            if !s.contains('/') {
                if let Ok(ip) = s.parse::<IpAddr>() {
                    let normalized = normalize_ip(ip);
                    let prefix = match normalized {
                        IpAddr::V4(_) => 32,
                        IpAddr::V6(_) => 128,
                    };
                    IpNet::new(normalized, prefix).ok()
                } else {
                    None
                }
            } else {
                if let Ok(net) = s.parse::<IpNet>() {
                    let normalized_addr = normalize_ip(net.addr());
                    IpNet::new(normalized_addr, net.prefix_len()).ok()
                } else {
                    None
                }
            }
        })
        .collect()
}

/// Check if a remote address is in the trusted proxies list
pub fn is_trusted_proxy(remote_addr: IpAddr, trusted_list: &[IpNet]) -> bool {
    let normalized = normalize_ip(remote_addr);
    for net in trusted_list {
        if net.contains(&normalized) {
            return true;
        }
    }
    false
}

/// Extract first IP from X-Forwarded-For header
pub fn first_ip_from_x_forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    let xff = headers.get("x-forwarded-for")?.to_str().ok()?;
    let first = xff.split(',').next()?.trim();
    first.parse::<IpAddr>().ok().map(normalize_ip)
}

/// Extract the secure real client IP address
pub fn get_client_ip(
    headers: &HeaderMap,
    connect_info: SocketAddr,
    trust_proxy: bool,
    trusted_proxies: &[IpNet],
) -> IpAddr {
    let socket_ip = normalize_ip(connect_info.ip());

    if !trust_proxy {
        return socket_ip;
    }

    if !trusted_proxies.is_empty() {
        if is_trusted_proxy(socket_ip, trusted_proxies) {
            if let Some(xff_ip) = first_ip_from_x_forwarded_for(headers) {
                return xff_ip;
            }
        }
        return socket_ip;
    }

    // Fall back to trusting X-Forwarded-For directly if trust_proxy is true but trusted_proxies is empty
    if let Some(xff_ip) = first_ip_from_x_forwarded_for(headers) {
        xff_ip
    } else {
        socket_ip
    }
}
