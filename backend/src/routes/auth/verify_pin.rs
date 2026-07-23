use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use shared_backend::auth::attempts;
use crate::ip::get_client_ip;
use std::net::SocketAddr;
use std::time::Duration;

use super::COOKIE_NAME;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct VerifyPinPayload {
    pub pin: String,
}

pub async fn verify_pin(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<VerifyPinPayload>,
) -> impl IntoResponse {
    let pin_req = &state.config.pin;
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
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );
    let max_attempts = state.config.max_attempts;
    let lockout_dur = Duration::from_secs(state.config.lockout_time_minutes * 60);

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

        let session_id = crate::session_id::generate_session_id();
        state
            .active_sessions
            .write()
            .await
            .insert(session_id.clone());

        let secure = crate::cookie_auth::cookie_should_be_secure(
            &headers,
            &state.config.base_url,
        );

        let cookie = crate::cookie_auth::build_cookie(&session_id,
            state.config.cookie_max_age_hours,
            secure,
        );
        let jar = jar.add(cookie);

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
