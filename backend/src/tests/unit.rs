use super::*;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use tokio::sync::RwLock;

#[test]
fn test_fuzzy_match_subsequence() {
    let score_exact = search::fuzzy_match_subsequence("log", "log");
    assert!(score_exact.is_some());

    let score_sub = search::fuzzy_match_subsequence("rapids", "rpd");
    assert!(score_sub.is_some());

    let score_none = search::fuzzy_match_subsequence("log", "xyz");
    assert!(score_none.is_none());
}

#[tokio::test]
async fn test_authenticated_no_pin_required() {
    use axum::http::HeaderMap;
    use axum_extra::extract::cookie::CookieJar;

    let config = AppConfig::load_from_env(4402);
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let jar = CookieJar::new();
    let headers = HeaderMap::new();

    assert!(routes::auth::is_authenticated(&jar, &state, &headers).await);
}

#[tokio::test]
async fn test_authenticated_with_valid_header_pin() {
    use axum::http::HeaderMap;
    use axum_extra::extract::cookie::CookieJar;

    let mut config = AppConfig::load_from_env(4402);
    config.pin = Some("1234".to_string());
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let jar = CookieJar::new();
    let mut headers = HeaderMap::new();
    headers.insert("x-pin", "1234".parse().unwrap());

    assert!(routes::auth::is_authenticated(&jar, &state, &headers).await);

    let mut invalid_headers = HeaderMap::new();
    invalid_headers.insert("x-pin", "9999".parse().unwrap());
    assert!(!routes::auth::is_authenticated(&jar, &state, &invalid_headers).await);
}

#[tokio::test]
async fn test_rate_limiter_budget_management() {
    let config = AppConfig::load_from_env(4402);
    let state: AppState = Arc::new(AppStateInner {
        config,
        data_dir: PathBuf::from("/tmp"),
        notepads_file: PathBuf::from("/tmp/notepads.json"),
        clients: RwLock::new(HashMap::new()),
        operations_history: RwLock::new(HashMap::new()),
        active_sessions: RwLock::new(HashSet::new()),
        rate_limiter: RwLock::new(HashMap::new()),
        notepads: RwLock::new(Vec::new()),
        index_items: RwLock::new(Vec::new()),
        notepads_lock: tokio::sync::Mutex::new(()),
    });
    let test_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    assert!(state.check_rate_limit(test_ip).await);
}

#[test]
fn test_origin_allowed_in_development() {
    assert!(ws::is_origin_allowed(
        Some("https://evil.example"),
        "http://localhost:4402",
        "development"
    ));
    assert!(ws::is_origin_allowed(
        None,
        "http://localhost:4402",
        "development"
    ));
}

#[test]
fn test_origin_allowed_in_production_matches_base() {
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com"),
        "https://pad.example.com",
        "production"
    ));
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com/"),
        "https://pad.example.com",
        "production"
    ));
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com"),
        "https://pad.example.com/",
        "production"
    ));
}

#[test]
fn test_origin_rejected_in_production() {
    assert!(!ws::is_origin_allowed(
        None,
        "https://pad.example.com",
        "production"
    ));
    assert!(!ws::is_origin_allowed(
        Some("https://evil.example"),
        "https://pad.example.com",
        "production"
    ));
    assert!(!ws::is_origin_allowed(
        Some("http://pad.example.com"),
        "https://pad.example.com",
        "production"
    ));
}

#[test]
fn test_origin_does_not_honor_wildcard_base_in_production() {
    assert!(!ws::is_origin_allowed(
        Some("https://anything"),
        "*",
        "production"
    ));
}
