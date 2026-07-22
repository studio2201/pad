use axum::{
    Router, middleware,
    routing::{get, post, put},
};
use shared_backend::middleware::{HstsState, cors_layer, hsts_layer, security_headers_layer};
use shared_backend::tracing_init::{default_log_dir, init_tracing};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

mod config;
mod routes;
pub mod services;
mod state;
#[cfg(test)]
mod tests;
mod ws;

pub use config::AppConfig;
use routes::*;
use services::migration::migrate_all_notepads_to_name_based_files;
pub use services::{migration, search};
use state::{AppState, AppStateInner};
use ws::handle_socket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_tracing(default_log_dir().as_deref());

    dotenvy::from_path("/app/data/.env").ok();
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(4402);

    let config = AppConfig::load_from_env(port);
    let site_title = config.server.site_title.clone();

    let root_path = PathBuf::from(".");
    let data_dir = std::env::var("PAD_DATA_DIR")
        .or_else(|_| std::env::var("DATA_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| root_path.join("data"));
    let notepads_file = data_dir.join("notepads.json");
    let public_dir = root_path.join("frontend/dist");

    // Initialize state. Note: `login_attempts` is intentionally absent — PIN
    // brute-force lockouts are now global via `shared_backend::auth::attempts`
    // and clean themselves up. We only manage the per-IP request budget here.
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir,
        notepads_file,
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(std::collections::HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
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
                    if let Some(ext) = path.extension()
                        && (ext == "txt" || path.file_name().is_some_and(|f| f == "notepads.json"))
                    {
                        should_reindex = true;
                        break;
                    }
                }
                if should_reindex {
                    state_clone.index_notepads().await;
                }
            }
        }
    });

    // Background cleanup for the per-IP request budget only. PIN-attempt
    // lockouts now live in `shared_backend::auth::attempts` (process-global)
    // and remove themselves when the lockout expires.
    let state_clone2 = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            state_clone2.clean_old_rate_limits().await;
        }
    });

    // CORS, security headers, and HSTS are all delegated to `shared-backend`,
    // so the same production-tested configuration applies across every
    // companion app. The `Arc<ServerConfig>` is shared between layers to keep
    // the dependency tree small.
    let server_config = Arc::new(state.config.server.clone());
    let cors = cors_layer(&server_config);

    // Setup routes
    let api_routes = Router::new()
        .route("/notepads", get(get_notepads).post(create_notepad))
        .route("/notepads/{id}", put(rename_notepad).delete(delete_notepad))
        .route("/notes/{id}", get(get_notes).post(save_notes))
        .route("/search", get(search_api))
        .layer(middleware::from_fn_with_state(state.clone(), require_pin));

    let public_api_routes = Router::new()
        .route("/verify-pin", post(verify_pin))
        .route("/pin-required", get(pin_required))
        .route("/config", get(get_config))
        .route("/logout", post(logout));

    let merged_api = api_routes
        .merge(public_api_routes)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::routes::rate_limit_middleware,
        ));

    let app = Router::new()
        .route("/", get(serve_root))
        .route("/login", get(serve_login))
        .route("/service-worker.js", get(serve_service_worker))
        .nest("/api", merged_api)
        .route("/ws", get(handle_socket))
        .route("/health", get(health_check))
        .fallback_service(
            ServeDir::new("frontend/dist")
                .precompressed_br()
                .precompressed_gzip(),
        )
        .layer(middleware::from_fn_with_state(
            HstsState(server_config.clone()),
            hsts_layer,
        ))
        .layer(middleware::from_fn(security_headers_layer))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Server is running on port {}", port);
    println!("Base URL: {}", state.config.server.base_url);
    println!("Environment: {}", state.config.node_env);
    println!("Version: {}", state.config.version);

    // Keep watcher in memory to prevent dropping it
    let _watcher_handle = watcher;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
