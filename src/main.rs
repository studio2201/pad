use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    middleware,
    response::{IntoResponse, Redirect},
    routing::{get, post, put},
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod migration;
mod state;
mod utils;
mod ws;

use migration::{
    get_notepad_file_path, migrate_all_notepads_to_name_based_files, sanitize_filename, Notepad,
};
use state::{AppConfig, AppState, AppStateInner, NotepadsJson};
use utils::{get_client_ip, parse_trusted_proxies, secure_compare};
use ws::handle_socket;

const COOKIE_NAME: &str = "dumbpad_auth";
const PAGE_HISTORY_COOKIE: &str = "dumbpad_page_history";

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(3000);
    let site_title = std::env::var("SITE_TITLE").unwrap_or_else(|_| "DumbPad".to_string());
    let pin = std::env::var("DUMBPAD_PIN").ok().filter(|s| !s.is_empty());
    
    // Validate PIN format
    let pin = pin.filter(|p| p.len() >= 4 && p.len() <= 10 && p.chars().all(|c| c.is_ascii_digit()));

    let cookie_max_age_hours = std::env::var("COOKIE_MAX_AGE")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(24);
    let page_history_cookie_age_days = std::env::var("PAGE_HISTORY_COOKIE_AGE")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(365);
    let max_attempts = std::env::var("MAX_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);
    let lockout_time_minutes = std::env::var("LOCKOUT_TIME")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(15);
    
    let trust_proxy = std::env::var("TRUST_PROXY").map(|s| s == "true").unwrap_or(false);
    let trusted_proxy_ips_raw = std::env::var("TRUSTED_PROXY_IPS").unwrap_or_default();
    let mut trusted_proxies = parse_trusted_proxies(&trusted_proxy_ips_raw);

    if trust_proxy && trusted_proxy_ips_raw.trim().is_empty() {
        eprintln!("CRITICAL WARNING: TRUST_PROXY=true but TRUSTED_PROXY_IPS is not set or empty.");
        eprintln!("Trust proxy is disabled for security. Set TRUSTED_PROXY_IPS to enable proxy trust.");
        trusted_proxies.clear();
    }

    let highlight_languages_raw = std::env::var("HIGHLIGHT_LANGUAGES").unwrap_or_default();
    let highlight_languages: Vec<String> = if highlight_languages_raw.trim().is_empty() {
        vec![
            "javascript".to_string(),
            "python".to_string(),
            "bash".to_string(),
            "css".to_string(),
            "html".to_string(),
            "rust".to_string(),
            "json".to_string(),
            "markdown".to_string(),
            "typescript".to_string(),
            "yaml".to_string(),
        ]
    } else {
        highlight_languages_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));
    let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());
    let version = env!("CARGO_PKG_VERSION").to_string();

    let root_path = PathBuf::from(".");
    let data_dir = root_path.join("data");
    let notepads_file = data_dir.join("notepads.json");
    let public_dir = root_path.join("public");

    // Initialize state
    let state: AppState = Arc::new(AppStateInner {
        config: AppConfig {
            port,
            site_title: site_title.clone(),
            pin: pin.clone(),
            cookie_max_age_hours,
            page_history_cookie_age_days,
            max_attempts,
            lockout_time_minutes,
            trust_proxy: trust_proxy && !trusted_proxy_ips_raw.trim().is_empty(),
            trusted_proxies,
            highlight_languages,
            base_url,
            node_env,
            version,
        },
        data_dir,
        notepads_file,
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        login_attempts: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
    });

    // Run migrations and load list
    if let Err(e) = state.ensure_data_dir().await {
        eprintln!("Error initializing data directory: {}", e);
        std::process::exit(1);
    }

    let notepads = state.load_notepads_list().await;
    migrate_all_notepads_to_name_based_files(&notepads, &state.data_dir).await;

    // Build PWA assets
    if let Err(e) = generate_pwa_manifest(&site_title, &public_dir) {
        eprintln!("Failed to generate manifests: {}", e);
    }

    // Initial search index
    state.index_notepads().await;

    // Start directory file watcher
    let state_clone = state.clone();
    let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::channel::<notify::Result<notify::Event>>(100);
    let mut watcher = notify::RecommendedWatcher::new(
        move |res| {
            let _ = watcher_tx.blocking_send(res);
        },
        notify::Config::default(),
    )
    .unwrap();

    use notify::Watcher;
    if let Err(e) = watcher.watch(&state.data_dir, notify::RecursiveMode::NonRecursive) {
        eprintln!("Failed to start file watcher on data directory: {}", e);
    }

    tokio::spawn(async move {
        while let Some(res) = watcher_rx.recv().await {
            if let Ok(event) = res {
                let mut should_reindex = false;
                for path in event.paths {
                    if let Some(ext) = path.extension() {
                        if ext == "txt" || path.file_name().map_or(false, |f| f == "notepads.json") {
                            should_reindex = true;
                            break;
                        }
                    }
                }
                if should_reindex {
                    state_clone.index_notepads().await;
                }
            }
        }
    });

    // Start background login lockout cleanup
    let state_clone2 = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            state_clone2.clean_old_lockouts().await;
        }
    });

    // CORS config
    let cors = tower_http::cors::CorsLayer::permissive(); // Setup CORS matching tower config if needed

    // Setup routes
    let api_routes = Router::new()
        .route("/notepads", get(get_notepads).post(create_notepad))
        .route("/notepads/:id", put(rename_notepad).delete(delete_notepad))
        .route("/notes/:id", get(get_notes).post(save_notes))
        .route("/search", get(search_api))
        .layer(middleware::from_fn_with_state(state.clone(), require_pin));

    let public_api_routes = Router::new()
        .route("/verify-pin", post(verify_pin))
        .route("/pin-required", get(pin_required))
        .route("/config", get(get_config));

    let app = Router::new()
        .route("/", get(serve_root))
        .route("/login", get(serve_login))
        .route("/service-worker.js", get(serve_service_worker))
        .nest("/api", api_routes.merge(public_api_routes))
        .route("/ws", get(handle_socket))
        .route("/health", get(health_check))
        // Serve highlight modules and marked modules dynamically mapped to node_modules/
        .nest_service("/js/marked", ServeDir::new("node_modules/marked/lib"))
        .nest_service(
            "/js/marked-extended-tables",
            ServeDir::new("node_modules/marked-extended-tables/src"),
        )
        .nest_service("/js/marked-alert", ServeDir::new("node_modules/marked-alert/dist"))
        .nest_service(
            "/js/marked-highlight",
            ServeDir::new("node_modules/marked-highlight/src"),
        )
        .nest_service(
            "/js/@highlightjs/languages",
            ServeDir::new("node_modules/@highlightjs/cdn-assets/es/languages"),
        )
        .nest_service("/js/@highlightjs", ServeDir::new("node_modules/@highlightjs/cdn-assets/es"))
        .nest_service(
            "/css/@highlightjs",
            ServeDir::new("node_modules/@highlightjs/cdn-assets/styles"),
        )
        .fallback_service(ServeDir::new("public"))
        .layer(cors)
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    println!("Server is running on port {}", port);
    println!("Base URL: {}", state.config.base_url);
    println!("Environment: {}", state.config.node_env);
    println!("Version: {}", state.config.version);

    // Keep watcher in memory to prevent dropping it
    let _watcher_handle = watcher;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

// Redirect URL validator helper
fn is_valid_redirect_url(url: &str) -> bool {
    if url.is_empty() || !url.starts_with('/') || url.starts_with("//") || url.contains('\\') {
        return false;
    }
    let lower = url.to_lowercase();
    if lower.contains("%2f") || lower.contains("%5c") {
        return false;
    }
    true
}

// Authenticated helper
fn is_authenticated(jar: &CookieJar, state: &AppState) -> bool {
    let pin = match &state.config.pin {
        Some(p) => p,
        None => return true,
    };
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        secure_compare(cookie.value(), pin)
    } else {
        false
    }
}

// Pin Middleware
async fn require_pin(
    jar: CookieJar,
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "Unauthorized" })),
        )
            .into_response();
    }
    next.run(req).await
}

// Root page server
async fn serve_root(
    jar: CookieJar,
    State(state): State<AppState>,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    if !is_authenticated(&jar, &state) {
        let redirect_param = percent_encoding::utf8_percent_encode(
            &uri.to_string(),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        return Redirect::temporary(&format!("/login?redirect={}", redirect_param)).into_response();
    }

    match fs::read_to_string(state.data_dir.parent().unwrap().join("public/index.html")).await {
        Ok(html) => ([(axum::http::header::CONTENT_TYPE, "text/html")], html).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading index.html: {}", e),
        )
            .into_response(),
    }
}

// Login page server
async fn serve_login(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if is_authenticated(&jar, &state) {
        if let Some(redirect) = params.get("redirect") {
            if is_valid_redirect_url(redirect) {
                return Redirect::temporary(redirect).into_response();
            }
        }
        return Redirect::temporary("/").into_response();
    }

    match fs::read_to_string(state.data_dir.parent().unwrap().join("public/login.html")).await {
        Ok(html) => ([(axum::http::header::CONTENT_TYPE, "text/html")], html).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error loading login.html: {}", e),
        )
            .into_response(),
    }
}

// Service worker serving
async fn serve_service_worker(State(state): State<AppState>) -> impl IntoResponse {
    let sw_path = state.data_dir.parent().unwrap().join("public/service-worker.js");
    match fs::read_to_string(&sw_path).await {
        Ok(content) => {
            // Replace APP_VERSION
            let re = regex::Regex::new(r#"let APP_VERSION = ".*?";"#).unwrap();
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
async fn health_check() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// API: Config
async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "siteTitle": state.config.site_title,
        "baseUrl": state.config.base_url,
        "version": state.config.version,
        "highlightLanguages": state.config.highlight_languages,
    }))
}

// API: PIN requirement check
async fn pin_required(
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

    let locked = state.is_locked_out(ip).await;
    axum::Json(serde_json::json!({
        "required": state.config.pin.is_some(),
        "length": state.config.pin.as_ref().map_or(0, |p| p.len()),
        "locked": locked
    }))
}

// API: Verify PIN
#[derive(serde::Deserialize)]
struct VerifyPinPayload {
    pin: String,
}

async fn verify_pin(
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

    // Verify correct format: numeric, length 4 to 10
    let is_valid_fmt = payload.pin.len() >= 4
        && payload.pin.len() <= 10
        && payload.pin.chars().all(|c| c.is_ascii_digit());

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

        let cookie_max_age = Duration::from_secs((state.config.cookie_max_age_hours * 3600) as u64);
        let same_site = SameSite::Strict;

        let secure = state.config.base_url.starts_with("https")
            && state.config.node_env == "production";

        let jar = jar.add(
            Cookie::build((COOKIE_NAME, payload.pin))
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

// API: List notepads
async fn get_notepads(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
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
async fn create_notepad(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let file_content = match fs::read_to_string(&state.notepads_file).await {
        Ok(c) => c,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "Error reading notepads file" })),
            )
                .into_response()
        }
    };

    let mut data: NotepadsJson = serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });
    
    let id = chrono::Utc::now().timestamp_millis().to_string();
    let desired_name = format!("Notepad {}", data.notepads.len() + 1);
    let unique_name = state.generate_unique_name(&desired_name, &data.notepads);

    let new_notepad = Notepad {
        id: id.clone(),
        name: unique_name.clone(),
    };
    data.notepads.push(new_notepad.clone());

    if let Err(_) = fs::write(&state.notepads_file, serde_json::to_string(&data).unwrap()).await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error updating notepads list" })),
        )
            .into_response();
    }

    let sanitized = sanitize_filename(&unique_name);
    let file_path = state.data_dir.join(format!("{}.txt", sanitized));
    if let Err(_) = fs::write(&file_path, "").await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error creating notepad file" })),
        )
            .into_response();
    }

    state.index_notepads().await;

    let secure = state.config.base_url.starts_with("https")
        && state.config.node_env == "production";
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
struct RenameNotepadPayload {
    name: String,
}

async fn rename_notepad(
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
                .into_response()
        }
    };

    let mut data: NotepadsJson = serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });
    
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
                .into_response()
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
    let sanitized_new = sanitize_filename(&unique_name);
    let mut new_file_path = state.data_dir.join(format!("{}.txt", sanitized_new));

    let should_rename_file = id != "default"
        && original_notepad.name != unique_name
        && current_file_path != new_file_path;

    if should_rename_file {
        // Handle collisions
        if fs::metadata(&new_file_path).await.is_ok() {
            let mut counter = 1;
            let mut found_available = false;
            while counter < 100 {
                let alt_name = sanitize_filename(&format!("{}-{}", unique_name, counter));
                let alt_path = state.data_dir.join(format!("{}.txt", alt_name));
                if fs::metadata(&alt_path).await.is_err() {
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

    if let Err(_) = fs::write(&state.notepads_file, serde_json::to_string(&data).unwrap()).await {
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

// API: Get notepad notes
async fn get_notes(
    Path(id): Path<String>,
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let list = state.notepads.read().await.clone();
    let notepad = list.into_iter().find(|n| n.id == id);

    let note_path = if let Some(n) = notepad {
        get_notepad_file_path(&n, &state.data_dir).await
    } else {
        let sanitized = sanitize_filename(&id);
        state.data_dir.join(format!("{}.txt", sanitized))
    };

    let content = fs::read_to_string(&note_path).await.unwrap_or_default();

    let secure = state.config.base_url.starts_with("https")
        && state.config.node_env == "production";
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

    (jar, axum::Json(serde_json::json!({ "content": content }))).into_response()
}

// API: Save notepad notes
#[derive(serde::Deserialize)]
struct SaveNotesPayload {
    content: String,
}

async fn save_notes(
    Path(id): Path<String>,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<SaveNotesPayload>,
) -> impl IntoResponse {
    let list = state.notepads.read().await.clone();
    let notepad = list.into_iter().find(|n| n.id == id);

    let note_path = if let Some(n) = notepad {
        get_notepad_file_path(&n, &state.data_dir).await
    } else {
        let sanitized = sanitize_filename(&id);
        state.data_dir.join(format!("{}.txt", sanitized))
    };

    if let Err(_) = fs::write(&note_path, &payload.content).await {
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
async fn delete_notepad(
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

    let file_content = match fs::read_to_string(&state.notepads_file).await {
        Ok(c) => c,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "Error reading notepads file" })),
            )
                .into_response()
        }
    };

    let mut data: NotepadsJson = serde_json::from_str(&file_content).unwrap_or(NotepadsJson { notepads: vec![] });
    
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
                .into_response()
        }
    };

    let deleted_notepad = data.notepads.remove(idx);
    
    if let Err(_) = fs::write(&state.notepads_file, serde_json::to_string_pretty(&data).unwrap()).await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "Error saving notepads list" })),
        )
            .into_response();
    }

    let file_path = get_notepad_file_path(&deleted_notepad, &state.data_dir).await;
    
    // Attempt deletion
    if fs::metadata(&file_path).await.is_ok() {
        let _ = fs::remove_file(&file_path).await;
    } else {
        // Try deleting legacy ID based file path
        let sanitized = sanitize_filename(&id);
        let legacy_path = state.data_dir.join(format!("{}.txt", sanitized));
        let _ = fs::remove_file(&legacy_path).await;
    }

    state.index_notepads().await;

    axum::Json(serde_json::json!({ "success": true, "message": "Notepad deleted successfully" })).into_response()
}

// API: Search
#[derive(serde::Deserialize)]
struct SearchQueryParams {
    query: Option<String>,
    page: Option<usize>,
}

async fn search_api(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let query = params.query.unwrap_or_default();
    let page = params.page.unwrap_or(1);

    let results = state.search_notepads(&query).await;
    let page_size = results.len(); // Defaults to returning all results in single page for now
    
    let total_pages = if page_size == 0 { 0 } else { (results.len() + page_size - 1) / page_size };
    let paginated_results = if page_size == 0 {
        vec![]
    } else {
        let start = (page - 1) * page_size;
        let end = std::cmp::min(page * page_size, results.len());
        if start < results.len() {
            results[start..end].to_vec()
        } else {
            vec![]
        }
    };

    axum::Json(serde_json::json!({
        "results": paginated_results,
        "totalPages": total_pages,
        "currentPage": page
    }))
}

// Recursive file scanner for Web App/Assets manifest generation
fn get_files(
    dir: &StdPath,
    base_path: &str,
    files: &mut Vec<String>,
) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name == ".DS_Store" || file_name == "Assets" {
                continue;
            }
            let sub_path = if base_path.is_empty() {
                format!("/{}", file_name)
            } else if base_path == "/" {
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

fn generate_pwa_manifest(site_title: &str, public_dir: &StdPath) -> std::io::Result<()> {
    let assets_dir = public_dir.join("Assets");
    std::fs::create_dir_all(&assets_dir)?;

    let mut files = Vec::new();
    get_files(public_dir, "", &mut files)?;
    
    let json_files = serde_json::to_string_pretty(&files)?;
    std::fs::write(assets_dir.join("asset-manifest.json"), json_files)?;

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
                "src": "dumbpad.png",
                "type": "image/png",
                "sizes": "192x192"
            },
            {
                "src": "dumbpad.png",
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
