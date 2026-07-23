pub mod logout;
pub mod pin_required;
pub mod verify_pin;

pub use logout::logout;
pub use pin_required::{get_config, pin_required};
pub use verify_pin::verify_pin;

use crate::state::AppState;
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;

use constant_time_eq::constant_time_eq;
use crate::ip::get_client_ip;
use std::net::SocketAddr;

pub const COOKIE_NAME: &str = "PAD_PIN";

// Authenticated helper
pub async fn is_authenticated(jar: &CookieJar, state: &AppState, headers: &HeaderMap) -> bool {
    let pin = match &state.config.pin {
        Some(p) => p,
        None => return true,
    };
    let cookie_pin = jar.get(COOKIE_NAME).map(|c| c.value());
    let header_pin = headers.get("x-pin").and_then(|h| h.to_str().ok());

    match (cookie_pin, header_pin) {
        (Some(cookie), _) => state.active_sessions.read().await.contains(cookie),
        (None, Some(hdr)) => constant_time_eq(hdr.as_bytes(), pin.as_bytes()),
        (None, None) => false,
    }
}

// Pin Middleware
pub async fn require_pin(
    jar: CookieJar,
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state, req.headers()).await {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "Unauthorized" })),
        )
            .into_response();
    }
    next.run(req).await
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    let addr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0);

    let ip = get_client_ip(
        req.headers(),
        addr.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0))),
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );
    let ip_key: std::net::IpAddr = ip
        .parse()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    if !state.check_rate_limit(ip_key).await {
        let body = serde_json::json!({
            "error": "Too many requests. Please slow down."
        });
        let mut response = axum::response::Json(body).into_response();
        *response.status_mut() = axum::http::StatusCode::TOO_MANY_REQUESTS;
        return response;
    }

    next.run(req).await
}
