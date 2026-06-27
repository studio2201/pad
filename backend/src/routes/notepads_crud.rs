use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::path::Path as StdPath;

use std::time::Duration;
use tokio::fs;

use crate::migration::{Notepad, get_notepad_file_path, sanitize_filename};
use crate::state::{AppState, NotepadsJson};

/// Returns `true` if `path` resolves to a location inside `data_dir`.
///
/// Used as the second line of defense after [`sanitize_filename`]: even if a
/// name passes the sanitizer, we confirm the resolved file path is contained
/// in the data directory before writing. Prevents symlink-in-data-dir escapes
/// (same class of bug the first review flagged in `beam`).
fn is_path_within_data_dir(path: &StdPath, data_dir: &StdPath) -> bool {
    let canonical_data = match data_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    // For files that don't exist yet (create/rename), canonicalize the parent
    // and re-attach the filename so we don't reject a legitimate new file.
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
                && let Ok(cp) = parent.canonicalize()
            {
                return cp.join(file_name).starts_with(&canonical_data);
            }
            return false;
        }
    };
    canonical_path.starts_with(&canonical_data)
}

pub const PAGE_HISTORY_COOKIE: &str = "log_page_history";

// API: List notepads
pub async fn get_notepads(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let list = state.notepads.read().await.clone();
    let note_history = jar
        .get(PAGE_HISTORY_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_else(|| "default".to_string());

    axum::Json(serde_json::json!({
        "notepads_list": list,
        "note_history": note_history
    }))
}

// API: Create new notepad
pub async fn create_notepad(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
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

    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();
    let desired_name = format!("Notepad {}", data.notepads.len() + 1);
    let unique_name = state.generate_unique_name(&desired_name, &data.notepads);

    let new_notepad = Notepad {
        id: id.clone(),
        name: unique_name.clone(),
    };
    data.notepads.push(new_notepad.clone());

    if fs::write(&state.notepads_file, serde_json::to_string(&data).unwrap())
        .await
        .is_err()
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error updating notepads list" })),
        )
            .into_response();
    }

    let sanitized = match sanitize_filename(&unique_name) {
        Ok(s) => s,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": format!("Invalid notepad name: {}", e) })),
            )
                .into_response();
        }
    };
    let file_path = state.data_dir.join(format!("{}.txt", sanitized));
    if !is_path_within_data_dir(&file_path, &state.data_dir) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": "Resolved path escapes data directory" })),
        )
            .into_response();
    }
    if fs::write(&file_path, "").await.is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error creating notepad file" })),
        )
            .into_response();
    }

    state.index_notepads().await;

    let secure =
        state.config.server.base_url.starts_with("https") && state.config.node_env == "production";
    let history_age_secs = (state.config.page_history_cookie_age_days * 24 * 3600) as u64;

    let jar = jar.add(
        Cookie::build((PAGE_HISTORY_COOKIE, id))
            .path("/")
            .http_only(true)
            .secure(secure)
            .same_site(SameSite::Strict)
            .max_age(Duration::from_secs(history_age_secs).try_into().unwrap())
            .build(),
    );

    (jar, axum::Json(new_notepad)).into_response()
}

// API: Rename notepad
#[derive(serde::Deserialize)]
pub struct RenameNotepadPayload {
    pub name: String,
}

pub async fn rename_notepad(
    Path(id): Path<String>,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<RenameNotepadPayload>,
) -> impl IntoResponse {
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

    let original_notepad = data.notepads[idx].clone();
    let other_notepads: Vec<Notepad> = data
        .notepads
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != idx)
        .map(|(_, n)| n.clone())
        .collect();

    let unique_name = state.generate_unique_name(&payload.name, &other_notepads);

    let current_file_path = get_notepad_file_path(&original_notepad, &state.data_dir).await;
    let sanitized_new = match sanitize_filename(&unique_name) {
        Ok(s) => s,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": format!("Invalid notepad name: {}", e) })),
            )
                .into_response();
        }
    };
    let mut new_file_path = state.data_dir.join(format!("{}.txt", sanitized_new));

    // Validate that the rename target stays inside the data directory. Defense
    // in depth: even if sanitize_filename regresses, this rejects any path
    // that would resolve outside `data_dir` (e.g. via a symlink).
    if !is_path_within_data_dir(&new_file_path, &state.data_dir) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": "Resolved path escapes data directory" })),
        )
            .into_response();
    }

    let should_rename_file = id != "default"
        && original_notepad.name != unique_name
        && current_file_path != new_file_path;

    if should_rename_file {
        if fs::metadata(&new_file_path).await.is_ok() {
            let mut counter = 1;
            let mut found_available = false;
            while counter < 100 {
                let alt_name = match sanitize_filename(&format!("{}-{}", unique_name, counter)) {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let alt_path = state.data_dir.join(format!("{}.txt", alt_name));
                if is_path_within_data_dir(&alt_path, &state.data_dir)
                    && fs::metadata(&alt_path).await.is_err()
                {
                    new_file_path = alt_path;
                    found_available = true;
                    break;
                }
                counter += 1;
            }
            if !found_available {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": "Unable to find available filename" })),
                )
                    .into_response();
            }
        }

        if let Err(e) = fs::rename(&current_file_path, &new_file_path).await {
            eprintln!("Failed to rename notepad file: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "Failed to rename notepad file" })),
            )
                .into_response();
        }
    }

    data.notepads[idx].name = unique_name.clone();

    if fs::write(&state.notepads_file, serde_json::to_string(&data).unwrap())
        .await
        .is_err()
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error updating notepads list" })),
        )
            .into_response();
    }

    state.index_notepads().await;

    axum::Json(serde_json::json!({
        "id": id,
        "name": unique_name,
        "nameChanged": unique_name != payload.name
    }))
    .into_response()
}
