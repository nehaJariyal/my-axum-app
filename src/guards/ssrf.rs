//! SSRF (Server-Side Request Forgery) protection.
//!
//! Use this module BEFORE your server fetches any user-supplied URL.
//! SSRF is not HTTP middleware — it belongs in helper/service layer.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

use reqwest::{redirect::Policy, Client};
use url::Url;

use crate::error::{AppError, AppResult};

const MAX_RESPONSE_BYTES: usize = 512 * 1024; // 512 KB
const FETCH_TIMEOUT_SECS: u64 = 5;

/// Step 1: Validate URL is safe to request (blocks internal/private targets).
pub fn validate_external_url(raw_url: &str) -> AppResult<()> {
    let url = Url::parse(raw_url)
        .map_err(|_| AppError::BadRequest("invalid url format".into()))?;

    // Block credentials in URL: http://user:pass@host
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::BadRequest(
            "url must not contain username or password".into(),
        ));
    }

    // Only http/https allowed (blocks file://, gopher://, ftp://, etc.)
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(AppError::BadRequest(
                "only http and https urls are allowed".into(),
            ));
        }
    }
  println!("{:?}",url);
    let host = url
        .host_str()
        .ok_or_else(|| AppError::BadRequest("url must contain a host".into()))?;

    validate_hostname(host)?;
    validate_resolved_ips(host, url.port_or_known_default().unwrap_or(80))?;

    Ok(())
}

/// Step 2: Safely fetch a validated URL (used for avatar preview, webhooks, etc.).
pub async fn fetch_safe_url(raw_url: &str) -> AppResult<FetchResult> {
    validate_external_url(raw_url)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        // Re-validate every redirect target to prevent SSRF via redirects.
        .redirect(Policy::custom(|attempt| {
            let next_url = attempt.url().as_str();
            match validate_external_url(next_url) {
                Ok(()) => attempt.follow(),
                Err(_) => attempt.stop(),
            }
        }))
        .build()
        .map_err(|_| AppError::BadRequest("failed to create http client".into()))?;

    let response = client
        .get(raw_url)
        .send()
        .await
        .map_err(|_| AppError::BadRequest("failed to fetch url".into()))?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|_| AppError::BadRequest("failed to read response body".into()))?;

    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(AppError::BadRequest(
            "response too large (max 512KB)".into(),
        ));
    }

    Ok(FetchResult {
        status,
        content_type,
        size_bytes: bytes.len(),
    })
}

#[derive(Debug, serde::Serialize)]
pub struct FetchResult {
    pub status: u16,
    pub content_type: String,
    pub size_bytes: usize,
}

fn validate_hostname(host: &str) -> AppResult<()> {
    let host_lower = host.to_lowercase();

    // Block obvious local hostnames.
    let blocked_hosts = [
        "localhost",
        "127.0.0.1",
        "0.0.0.0",
        "::1",
        "metadata.google.internal",
        "metadata",
    ];
    if blocked_hosts.contains(&host_lower.as_str()) {
        return Err(AppError::BadRequest(
            "local or internal hostnames are not allowed".into(),
        ));
    }

    if host_lower.ends_with(".local") || host_lower.ends_with(".internal") {
        return Err(AppError::BadRequest(
            "internal domain suffix is not allowed".into(),
        ));
    }

    // Block direct IP literals (including decimal/octal/hex encodings).
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(AppError::BadRequest(
                "url points to a private or internal network".into(),
            ));
        }
    }

    Ok(())
}

fn validate_resolved_ips(host: &str, port: u16) -> AppResult<()> {
    let socket_addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|_| AppError::BadRequest("unable to resolve url host".into()))?
        .collect();

    if socket_addrs.is_empty() {
        return Err(AppError::BadRequest("unable to resolve url host".into()));
    }

    for addr in socket_addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(AppError::BadRequest(
                "url resolves to a private or internal network".into(),
            ));
        }
    }

    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
        }
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.octets()[0] == 169 && ip.octets()[1] == 254 // AWS/GCP metadata
}
