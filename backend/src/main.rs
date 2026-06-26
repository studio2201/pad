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
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod migration;
mod routes;
mod search;
mod state;
#[cfg(test)]
mod tests;
mod utils;
mod ws;

pub use config::AppConfig;
use migration::migrate_all_notepads_to_name_based_files;
use routes::*;
use state::{AppState, AppStateInner};
use ws::handle_socket;

#[tokio::main]
async fn main() {
    // Initialize logging
    let log_dir = std::env::var("LOG_DIR").ok().or_else(|| {
        let data_dir = std::path::Path::new("/app/data");
        if data_dir.is_dir() {
            Some("/app/data/log".to_string())
        } else {
            Some("/app/log".to_string())
        }
    });

    let (file_layer_error, file_layer_app) = if let Some(ref dir) = log_dir {
        if dir == "off" || dir == "none" || dir == "false" {
            (None, None)
        } else {
            let _ = std::fs::create_dir_all(dir);
            let error_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(std::path::Path::new(dir).join("error.log"))
                .ok();
            let app_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(std::path::Path::new(dir).join("app.log"))
                .ok();

            let error_layer = error_file.map(|file| {
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::WARN)
            });

            let app_layer = app_file.map(|file| {
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false)
                    .with_filter(tracing_subscriber::filter::LevelFilter::INFO)
            });

            (error_layer, app_layer)
        }
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer_error)
        .with(file_layer_app)
        .init();

    dotenvy::from_path("/app/data/.env").ok();
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(4402);

    let config = AppConfig::load_from_env(port);
    let site_title = config.site_title.clone();

    let root_path = PathBuf::from(".");
    let data_dir = root_path.join("data");
    let notepads_file = data_dir.join("notepads.json");
    let public_dir = root_path.join("frontend/dist");

    // Initialize state
    let state: AppState = Arc::new(AppStateInner {
        config,
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

    let cors = if state.config.allowed_origins == "*" {
        tower_http::cors::CorsLayer::permissive()
    } else {
        let mut cors = tower_http::cors::CorsLayer::new()
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::COOKIE]);
        for origin in state.config.allowed_origins.split(',') {
            if let Ok(parsed) = origin.trim().parse::<axum::http::HeaderValue>() {
                cors = cors.allow_origin(parsed);
            }
        }
        cors.allow_credentials(true)
    };

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
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(cors)
        .layer(middleware::from_fn(security_headers_middleware))
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
