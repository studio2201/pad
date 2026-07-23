use axum::{
    extract::{Query, State},
    http::Uri,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use std::collections::HashMap;
use tokio::fs;

use crate::routes::auth::is_authenticated;
use crate::state::AppState;

// Redirect URL validator helper
pub fn is_valid_redirect_url(url: &str) -> bool {
    if url.is_empty() || !url.starts_with('/') || url.starts_with("//") || url.contains('\\') {
        return false;
    }
    let lower = url.to_lowercase();
    !lower.contains("%2f") && !lower.contains("%5c")
}

// Root page server
pub async fn serve_root(
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state, &headers).await {
        let redirect_param = percent_encoding::utf8_percent_encode(
            &uri.to_string(),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        return Redirect::temporary(&format!("/login?redirect={}", redirect_param)).into_response();
    }

    match fs::read_to_string(
        state
            .data_dir
            .parent()
            .unwrap_or(&state.data_dir)
            .join("frontend/dist/index.html"),
    )
    .await
    {
        Ok(html) => {
            let rendered = html.replace("{{SITE_TITLE}}", &state.config.site_title);
            ([(axum::http::header::CONTENT_TYPE, "text/html")], rendered).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading index.html: {}", e),
        )
            .into_response(),
    }
}

// Login page server
pub async fn serve_login(
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if is_authenticated(&jar, &state, &headers).await {
        if let Some(redirect) = params.get("redirect")
            && is_valid_redirect_url(redirect)
        {
            return Redirect::temporary(redirect).into_response();
        }
        return Redirect::temporary("/").into_response();
    }

    match fs::read_to_string(
        state
            .data_dir
            .parent()
            .unwrap_or(&state.data_dir)
            .join("frontend/dist/index.html"),
    )
    .await
    {
        Ok(html) => {
            let rendered = html.replace("{{SITE_TITLE}}", &state.config.site_title);
            ([(axum::http::header::CONTENT_TYPE, "text/html")], rendered).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading index.html: {}", e),
        )
            .into_response(),
    }
}
