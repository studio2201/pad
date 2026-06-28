use super::COOKIE_NAME;
use crate::state::AppState;
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use shared_assets::auth::attempts;
use shared_assets::server::get_client_ip;
use std::net::SocketAddr;
use std::time::Duration;

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

pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "siteTitle": state.config.server.site_title,
        "baseUrl": state.config.server.base_url,
        "version": state.config.version,
        "enableTranslation": state.config.server.enable_translation,
        "enable_translation": state.config.server.enable_translation,
        "enableThemes": state.config.server.enable_themes,
        "enable_themes": state.config.server.enable_themes,
        "enablePrint": state.config.server.enable_print,
        "enable_print": state.config.server.enable_print,
        "showVersion": state.config.server.show_version,
        "show_version": state.config.server.show_version,
        "showGithub": state.config.server.show_github,
        "show_github": state.config.server.show_github,
    }))
}

pub async fn pin_required(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ip_str = get_client_ip(
        &headers,
        addr,
        state.config.server.trust_proxy,
        &state.config.server.trusted_proxies,
    );
    let lockout_dur = Duration::from_secs(state.config.server.lockout_time_minutes * 60);
    axum::Json(serde_json::json!({
        "required": state.config.server.pin.is_some(),
        "length": state.config.server.pin.as_ref().map_or(0, |p| p.len()),
        "locked": attempts::is_locked_out(&ip_str, state.config.server.max_attempts, lockout_dur),
        "enable_translation": state.config.server.enable_translation,
        "enable_themes": state.config.server.enable_themes,
        "enable_print": state.config.server.enable_print,
        "show_version": state.config.server.show_version,
        "show_github": state.config.server.show_github,
    }))
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

    let expected_pin = pin_req.as_ref().unwrap();

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
                .max_age(cookie_max_age.try_into().unwrap())
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

pub async fn logout(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        state.active_sessions.write().await.remove(cookie.value());
    }
    let jar = jar.add(
        Cookie::build((COOKIE_NAME, ""))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Strict)
            .max_age(Duration::from_secs(0).try_into().unwrap())
            .build(),
    );
    (jar, axum::Json(serde_json::json!({ "success": true }))).into_response()
}
