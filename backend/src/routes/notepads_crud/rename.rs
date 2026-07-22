use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use tokio::fs;

use super::helper::is_path_within_data_dir;
use crate::migration::{Notepad, get_notepad_file_path, sanitize_filename};
use crate::state::{AppState, NotepadsJson};

#[derive(serde::Deserialize)]
pub struct RenameNotepadPayload {
    pub name: String,
}

pub async fn rename_notepad(
    Path(id): Path<String>,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<RenameNotepadPayload>,
) -> impl IntoResponse {
    let unique_name = {
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
                    axum::Json(
                        serde_json::json!({ "error": format!("Invalid notepad name: {}", e) }),
                    ),
                )
                    .into_response();
            }
        };
        let mut new_file_path = state.data_dir.join(format!("{}.txt", sanitized_new));

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
                    let alt_name = match sanitize_filename(&format!("{}-{}", unique_name, counter))
                    {
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
                        axum::Json(
                            serde_json::json!({ "error": "Unable to find available filename" }),
                        ),
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
        unique_name
    };

    state.index_notepads().await;

    axum::Json(serde_json::json!({
        "id": id,
        "name": unique_name,
        "nameChanged": unique_name != payload.name
    }))
    .into_response()
}
