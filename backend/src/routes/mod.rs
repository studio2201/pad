pub mod auth;
pub mod notepads_crud;
pub mod notepads_io;
pub mod pages;

pub use auth::{get_config, logout, pin_required, rate_limit_middleware, require_pin, verify_pin};
pub use notepads_crud::{create_notepad, get_notepads, rename_notepad};
pub use notepads_io::{delete_notepad, get_notes, save_notes};
pub use pages::{serve_login, serve_root};

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use std::path::Path as StdPath;
use tokio::fs;

use crate::state::AppState;

/// Default page size for the search API. 25 fits comfortably in the viewport
/// without forcing an extra round-trip for typical notepad collections.
const SEARCH_PAGE_SIZE: usize = 25;

// API: Search
#[derive(serde::Deserialize)]
pub struct SearchQueryParams {
    pub query: Option<String>,
    pub page: Option<usize>,
}

pub async fn search_api(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let query = params.query.unwrap_or_default();
    // Clamp `page` to a sane range; downstream math assumes 1-indexed positive.
    let page = params.page.unwrap_or(1).max(1);

    let results = state.search_notepads(&query).await;
    let total = results.len();
    let total_pages = total.div_ceil(SEARCH_PAGE_SIZE).max(1);
    // If the caller requests a page past the end, return an empty result set
    // rather than 400 — keeps the UI responsive while typing fast queries.
    let page = page.min(total_pages);

    let start = (page - 1) * SEARCH_PAGE_SIZE;
    let end = (start + SEARCH_PAGE_SIZE).min(total);
    let paginated_results = if total == 0 || start >= total {
        Vec::new()
    } else {
        results[start..end].to_vec()
    };

    axum::Json(serde_json::json!({
        "results": paginated_results,
        "totalPages": total_pages,
        "currentPage": page,
        "pageSize": SEARCH_PAGE_SIZE,
        "total": total,
    }))
}

// Service worker serving
pub async fn serve_service_worker(State(state): State<AppState>) -> impl IntoResponse {
    let parent_dir = state.data_dir.parent().unwrap_or(&state.data_dir);
    let sw_path = parent_dir.join("frontend/dist/service-worker.js");
    match fs::read_to_string(&sw_path).await {
        Ok(content) => {
            let re = match regex::Regex::new(r#"let APP_VERSION = ".*?";"#) {
                Ok(r) => r,
                Err(_) => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Regex compilation error".to_string(),
                    )
                        .into_response();
                }
            };
            let replacement = format!(r#"let APP_VERSION = "{}";"#, state.config.version);
            let updated = re.replace(&content, replacement.as_str()).to_string();

            (
                [
                    (axum::http::header::CONTENT_TYPE, "application/javascript"),
                    (
                        axum::http::header::CACHE_CONTROL,
                        "no-cache, no-store, must-revalidate",
                    ),
                    (axum::http::header::PRAGMA, "no-cache"),
                    (axum::http::header::EXPIRES, "0"),
                ],
                updated,
            )
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading service-worker.js: {}", e),
        )
            .into_response(),
    }
}

// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": secs
    }))
}

// Recursive file scanner for Web App/Assets manifest generation
fn get_files(dir: &StdPath, base_path: &str, files: &mut Vec<String>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name == ".DS_Store" || file_name == "assets" {
                continue;
            }
            let sub_path = if base_path.is_empty() || base_path == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", base_path, file_name)
            };
            if path.is_dir() {
                get_files(&path, &sub_path, files)?;
            } else {
                files.push(sub_path);
            }
        }
    }
    Ok(())
}

pub fn generate_pwa_manifest(site_title: &str, public_dir: &StdPath) -> std::io::Result<()> {
    let assets_dir = public_dir.join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    let mut files = Vec::new();
    get_files(public_dir, "", &mut files)?;

    let json_files = serde_json::to_string_pretty(&files)?;
    std::fs::write(public_dir.join("asset-manifest.json"), json_files)?;

    let pwa_manifest = serde_json::json!({
        "name": site_title,
        "short_name": site_title,
        "description": "A simple notepad application",
        "start_url": "/",
        "display": "standalone",
        "background_color": "#ffffff",
        "theme_color": "#000000",
        "icons": [
            {
                "src": "log.png",
                "type": "image/png",
                "sizes": "192x192"
            },
            {
                "src": "log.png",
                "type": "image/png",
                "sizes": "512x512"
            }
        ],
        "orientation": "any"
    });
    let json_pwa = serde_json::to_string_pretty(&pwa_manifest)?;
    std::fs::write(assets_dir.join("manifest.json"), json_pwa)?;

    println!("Asset and PWA manifests generated dynamically!");
    Ok(())
}
