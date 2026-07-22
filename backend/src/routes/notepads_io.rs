use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::time::Duration;
use tokio::fs;

use crate::migration::{get_notepad_file_path, sanitize_filename};
use crate::state::{AppState, NotepadsJson};

pub const PAGE_HISTORY_COOKIE: &str = "log_page_history";

// API: Get notepad notes
pub async fn get_notes(
    Path(id): Path<String>,
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let list = state.notepads.read().await.clone();
    let notepad = list.into_iter().find(|n| n.id == id);

    let note_path = if let Some(n) = notepad {
        get_notepad_file_path(&n, &state.data_dir).await
    } else {
        let sanitized = sanitize_filename(&id).unwrap_or_else(|_| "unnamed".to_string());
        state.data_dir.join(format!("{}.txt", sanitized))
    };

    let content = fs::read_to_string(&note_path).await.unwrap_or_default();

    let secure =
        state.config.server.base_url.starts_with("https") && state.config.node_env == "production";
    let history_age_secs = (state.config.page_history_cookie_age_days * 24 * 3600) as u64;

    let jar = jar.add(
        Cookie::build((PAGE_HISTORY_COOKIE, id))
            .path("/")
            .http_only(true)
            .secure(secure)
            .same_site(SameSite::Strict)
            .max_age(
                Duration::from_secs(history_age_secs)
                    .try_into()
                    .unwrap_or_default(),
            )
            .build(),
    );

    (jar, axum::Json(serde_json::json!({ "content": content }))).into_response()
}

// API: Save notepad notes
#[derive(serde::Deserialize)]
pub struct SaveNotesPayload {
    pub content: String,
}

pub async fn save_notes(
    Path(id): Path<String>,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<SaveNotesPayload>,
) -> impl IntoResponse {
    let list = state.notepads.read().await.clone();
    let notepad = list.into_iter().find(|n| n.id == id);

    let note_path = if let Some(n) = notepad {
        get_notepad_file_path(&n, &state.data_dir).await
    } else {
        let sanitized = sanitize_filename(&id).unwrap_or_else(|_| "unnamed".to_string());
        state.data_dir.join(format!("{}.txt", sanitized))
    };

    if fs::write(&note_path, &payload.content).await.is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error saving notes" })),
        )
            .into_response();
    }

    state.index_notepads().await;

    axum::Json(serde_json::json!({ "success": true })).into_response()
}

// API: Delete notepad
pub async fn delete_notepad(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if id == "default" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": "Cannot delete default notepad" })),
        )
            .into_response();
    }

    let deleted_notepad = {
        let _lock = state.notepads_lock.lock().await;

        let file_content = match fs::read_to_string(&state.notepads_file).await {
            Ok(c) => c,
            Err(_) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": "Error reading notepads file" })),
                )
                    .into_response();
            }
        };

        let mut data: NotepadsJson =
            serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });

        let mut notepad_idx = None;
        for (i, n) in data.notepads.iter().enumerate() {
            if n.id == id {
                notepad_idx = Some(i);
                break;
            }
        }

        let idx = match notepad_idx {
            Some(i) => i,
            None => {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({ "error": "Notepad not found" })),
                )
                    .into_response();
            }
        };

        let deleted_notepad = data.notepads.remove(idx);

        let json_str = match serde_json::to_string_pretty(&data) {
            Ok(s) => s,
            Err(_) => {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": "Error serializing notepads list" })),
                )
                    .into_response();
            }
        };

        if fs::write(&state.notepads_file, &json_str).await.is_err() {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "Error saving notepads list" })),
            )
                .into_response();
        }
        deleted_notepad
    };

    let file_path = get_notepad_file_path(&deleted_notepad, &state.data_dir).await;

    if fs::metadata(&file_path).await.is_ok() {
        let _ = fs::remove_file(&file_path).await;
    } else {
        let sanitized = sanitize_filename(&id).unwrap_or_else(|_| "unnamed".to_string());
        let legacy_path = state.data_dir.join(format!("{}.txt", sanitized));
        let _ = fs::remove_file(&legacy_path).await;
    }

    state.index_notepads().await;

    axum::Json(serde_json::json!({ "success": true, "message": "Notepad deleted successfully" }))
        .into_response()
}
