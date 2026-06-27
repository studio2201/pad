//! Pad-specific configuration layered on top of shared [`ServerConfig`].
//!
//! Pad adds three fields beyond the shared baseline:
//! - `page_history_cookie_age_days` — undo-history persistence
//! - `node_env` — dev/prod env hint
//! - `version` — CARGO_PKG_VERSION snapshot

use shared_assets::server::ServerConfig;

/// Pad application configuration. Wraps [`ServerConfig`] with pad-specific
/// retention and version fields.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub page_history_cookie_age_days: i64,
    pub node_env: String,
    pub version: String,
}

impl AppConfig {
    /// Build a config by combining shared [`ServerConfig::from_env`] with
    /// pad-specific env parsing.
    pub fn load_from_env(port: u16) -> Self {
        let mut server = ServerConfig::from_env("PAD");
        // Pad's load_from_env signature takes port; override the shared default
        // with the caller's value if it differs from the default.
        if server.port == 4401 && port != 4401 {
            server.port = port;
        }

        let page_history_cookie_age_days = std::env::var("PAGE_HISTORY_COOKIE_AGE")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(365);

        let node_env = std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());

        Self {
            server,
            page_history_cookie_age_days,
            node_env,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
