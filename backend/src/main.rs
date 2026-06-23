use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod migration;
mod routes;
mod search;
mod state;
#[cfg(test)]
mod tests;
mod utils;
mod ws;

use migration::migrate_all_notepads_to_name_based_files;
use routes::*;
use state::{AppConfig, AppState, AppStateInner};
use utils::parse_trusted_proxies;
use ws::handle_socket;

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
        .unwrap_or(4402);
    let site_title = std::env::var("RUSTPAD_TITLE")
        .or_else(|_| std::env::var("RUSTPAD_SITE_TITLE"))
        .or_else(|_| std::env::var("SITE_TITLE"))
        .unwrap_or_else(|_| "RustPad".to_string());
    let pin = std::env::var("RUSTPAD_PIN")
        .ok()
        .filter(|s| !s.is_empty())
        .filter(|p| p.len() >= 4 && p.len() <= 10 && p.chars().all(|c| c.is_ascii_digit()));

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

    let trust_proxy = std::env::var("TRUST_PROXY")
        .map(|s| s == "true")
        .unwrap_or(false);
    let trusted_proxy_ips_raw = std::env::var("TRUSTED_PROXY_IPS").unwrap_or_default();
    let mut trusted_proxies = parse_trusted_proxies(&trusted_proxy_ips_raw);

    if trust_proxy && trusted_proxy_ips_raw.trim().is_empty() {
        eprintln!("CRITICAL WARNING: TRUST_PROXY=true but TRUSTED_PROXY_IPS is not set or empty.");
        eprintln!(
            "Trust proxy is disabled for security. Set TRUSTED_PROXY_IPS to enable proxy trust."
        );
        trusted_proxies.clear();
    }

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));
    let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());
    let version = env!("CARGO_PKG_VERSION").to_string();

    let root_path = PathBuf::from(".");
    let data_dir = root_path.join("data");
    let notepads_file = data_dir.join("notepads.json");
    let public_dir = root_path.join("frontend/dist");

    // Initialize state
    let state: AppState = Arc::new(AppStateInner {
        config: AppConfig {
            site_title: site_title.clone(),
            pin: pin.clone(),
            cookie_max_age_hours,
            page_history_cookie_age_days,
            max_attempts,
            lockout_time_minutes,
            trust_proxy: trust_proxy && !trusted_proxy_ips_raw.trim().is_empty(),
            trusted_proxies,
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
    let (watcher_tx, mut watcher_rx) =
        tokio::sync::mpsc::channel::<notify::Result<notify::Event>>(100);
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
                        if ext == "txt" || path.file_name().is_some_and(|f| f == "notepads.json") {
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

    let cors = tower_http::cors::CorsLayer::permissive();

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
        .route("/config", get(get_config))
        .route("/logout", post(logout));

    let app = Router::new()
        .route("/", get(serve_root))
        .route("/login", get(serve_login))
        .route("/service-worker.js", get(serve_service_worker))
        .nest("/api", api_routes.merge(public_api_routes))
        .route("/ws", get(handle_socket))
        .route("/health", get(health_check))
        .fallback_service(
            ServeDir::new("frontend/dist")
                .precompressed_br()
                .precompressed_gzip(),
        )
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
