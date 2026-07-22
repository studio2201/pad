use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use shared_backend::auth::attempts;
use shared_backend::server::get_client_ip;
use std::net::SocketAddr;
use std::time::Duration;

use super::COOKIE_NAME;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct VerifyPinPayload {
    pub pin: String,
}

pub fn generate_session_id() -> String {
    use std::fs::File;
    use std::io::Read;
    let file = File::open("/dev/urandom").ok();
    let mut bytes = [0u8; 16];
    if let Some(mut f) = file
        && f.read_exact(&mut bytes).is_ok()
    {
        return bytes.iter().map(|b| format!("{:02x}", b)).collect();
    }
    let random_val = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(random_val.to_string().as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

pub async fn verify_pin(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<VerifyPinPayload>,
) -> impl IntoResponse {
    let pin_req = &state.config.server.pin;
    if pin_req.is_none() {
        return (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({ "success": true })),
        )
            .into_response();
    }

    let ip_str = get_client_ip(
        &headers,
        addr,
        state.config.server.trust_proxy,
        &state.config.server.trusted_proxies,
    );
    let max_attempts = state.config.server.max_attempts;
    let lockout_dur = Duration::from_secs(state.config.server.lockout_time_minutes * 60);

    if attempts::is_locked_out(&ip_str, max_attempts, lockout_dur) {
        let remaining = attempts::lockout_remaining_secs(&ip_str, lockout_dur);
        let time_left_min = (remaining as f64 / 60.0).ceil() as u64;

        return (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({
                "error": format!("Too many attempts. Please try again in {} minute(s).", time_left_min)
            })),
        )
            .into_response();
    }

    let expected_pin = match pin_req.as_ref() {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({ "success": true })),
            )
                .into_response();
        }
    };

    let is_valid_fmt = payload.pin.len() >= 4 && payload.pin.len() <= 64;

    if !is_valid_fmt {
        attempts::record_attempt(&ip_str);
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN format"
            })),
        )
            .into_response();
    }

    if constant_time_eq::constant_time_eq(payload.pin.as_bytes(), expected_pin.as_bytes()) {
        attempts::reset_attempts(&ip_str);

        let session_id = generate_session_id();
        state
            .active_sessions
            .write()
            .await
            .insert(session_id.clone());

        let cookie_max_age =
            Duration::from_secs((state.config.server.cookie_max_age_hours * 3600) as u64);
        let same_site = SameSite::Strict;

        let secure = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("https"))
            .unwrap_or_else(|| state.config.server.base_url.starts_with("https"));

        let jar = jar.add(
            Cookie::build((COOKIE_NAME, session_id))
                .path("/")
                .http_only(true)
                .secure(secure)
                .same_site(same_site)
                .max_age(cookie_max_age.try_into().unwrap_or_default())
                .build(),
        );

        (jar, axum::Json(serde_json::json!({ "success": true }))).into_response()
    } else {
        let attempt = attempts::record_attempt(&ip_str);
        let attempts_left = max_attempts.saturating_sub(attempt.count);

        (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN",
                "attemptsLeft": attempts_left
            })),
        )
            .into_response()
    }
}
