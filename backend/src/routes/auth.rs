use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::net::SocketAddr;
use std::time::Duration;

use crate::state::AppState;
use crate::utils::{get_client_ip, secure_compare};

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
        (None, Some(hdr)) => secure_compare(hdr, pin),
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

// API: Config
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "siteTitle": state.config.site_title,
        "baseUrl": state.config.base_url,
        "version": state.config.version,
        "enableTranslation": state.config.enable_translation,
        "enable_translation": state.config.enable_translation,
        "enableThemes": state.config.enable_themes,
        "enable_themes": state.config.enable_themes,
        "enablePrint": state.config.enable_print,
        "enable_print": state.config.enable_print,
        "showVersion": state.config.show_version,
        "show_version": state.config.show_version,
        "showGithub": state.config.show_github,
        "show_github": state.config.show_github,
    }))
}

// API: PIN requirement check
pub async fn pin_required(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ip = get_client_ip(
        &headers,
        addr,
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );
    axum::Json(serde_json::json!({
        "required": state.config.pin.is_some(),
        "length": state.config.pin.as_ref().map_or(0, |p| p.len()),
        "locked": state.is_locked_out(ip).await,
        "enable_translation": state.config.enable_translation,
        "enable_themes": state.config.enable_themes,
        "enable_print": state.config.enable_print,
        "show_version": state.config.show_version,
        "show_github": state.config.show_github,
    }))
}

// API: Verify PIN
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
    let pin_req = &state.config.pin;
    if pin_req.is_none() {
        return (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({ "success": true })),
        )
            .into_response();
    }

    let ip = get_client_ip(
        &headers,
        addr,
        state.config.trust_proxy,
        &state.config.trusted_proxies,
    );

    if state.is_locked_out(ip).await {
        let map = state.login_attempts.read().await;
        let last_time = map.get(&ip).map(|a| a.last_attempt).unwrap();
        let lockout_dur = Duration::from_secs(state.config.lockout_time_minutes * 60);
        let time_left = lockout_dur
            .checked_sub(last_time.elapsed())
            .unwrap_or(Duration::ZERO);
        let time_left_min = (time_left.as_secs_f64() / 60.0).ceil() as u64;

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
        state.record_login_attempt(ip).await;
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Invalid PIN format"
            })),
        )
            .into_response();
    }

    if secure_compare(&payload.pin, expected_pin) {
        state.reset_login_attempts(ip).await;

        let session_id = generate_session_id();
        state
            .active_sessions
            .write()
            .await
            .insert(session_id.clone());

        let cookie_max_age = Duration::from_secs((state.config.cookie_max_age_hours * 3600) as u64);
        let same_site = SameSite::Strict;

        let secure = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("https"))
            .unwrap_or_else(|| state.config.base_url.starts_with("https"));

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
        state.record_login_attempt(ip).await;

        let map = state.login_attempts.read().await;
        let attempts_count = map.get(&ip).map(|a| a.count).unwrap_or(0);
        let attempts_left = state.config.max_attempts.saturating_sub(attempts_count);

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

// API: Logout
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

    if !state.check_rate_limit(ip).await {
        let body = serde_json::json!({
            "error": "Too many requests. Please slow down."
        });
        let mut response = axum::response::Json(body).into_response();
        *response.status_mut() = axum::http::StatusCode::TOO_MANY_REQUESTS;
        return response;
    }

    next.run(req).await
}

pub async fn security_headers_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        "X-Frame-Options",
        axum::http::header::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "X-Content-Type-Options",
        axum::http::header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "Referrer-Policy",
        axum::http::header::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "Content-Security-Policy", 
        axum::http::header::HeaderValue::from_static(
            "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src 'self' data: blob: https:; connect-src 'self' ws: wss: http: https:; font-src 'self'; manifest-src 'self';"
        )
    );

    response
}
