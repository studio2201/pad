use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::time::Duration;

use super::COOKIE_NAME;
use crate::state::AppState;

pub async fn logout(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        state.active_sessions.write().await.remove(cookie.value());
    }
    let jar = jar.add(
        Cookie::build((COOKIE_NAME, ""))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Strict)
            .max_age(Duration::from_secs(0).try_into().unwrap_or_default())
            .build(),
    );
    (jar, axum::Json(serde_json::json!({ "success": true }))).into_response()
}
