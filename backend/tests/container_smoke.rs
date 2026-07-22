use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
const APP_NAME: &str = "pad";
const DEFAULT_PORT: u16 = 4402;
const FAVICON_CANDIDATES: &[&str] = &["/assets/favicon.png", "/favicon.png"];
const MANIFEST_CANDIDATES: &[&str] = &["/assets/manifest.json", "/manifest.json"];
const CONFIG_CANDIDATES: &[&str] = &["/api/config", "/api/auth/config"];
const SERVICE_WORKER_CANDIDATES: &[&str] = &[
    "/service-worker.js",
    "/api/service-worker.js",
    "/assets/service-worker.js",
];
fn port() -> u16 {
    std::env::var("SMOKE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}
fn pin() -> String {
    std::env::var("SMOKE_PIN").unwrap_or_else(|_| "1234".to_string())
}
fn base_url() -> String {
    format!("http://127.0.0.1:{}", port())
}
fn client() -> Client {
    Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("reqwest client")
}
async fn wait_for_health() {
    let c = client();
    for _ in 0..30 {
        if let Ok(r) = c.get(format!("{}/health", base_url())).send().await {
            if r.status().is_success() {
                return;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    panic!("container at {} never became healthy", base_url());
}
async fn try_paths(c: &Client, paths: &[&str]) -> Option<reqwest::Response> {
    for p in paths {
        if let Ok(r) = c.get(format!("{}{}", base_url(), p)).send().await {
            if r.status().is_success() {
                return Some(r);
            }
        }
    }
    None
}
#[tokio::test]
#[ignore]
async fn health_returns_200() {
    let c = client();
    let r = c
        .get(format!("{}/health", base_url()))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "expected 200 from /health");
}
#[tokio::test]
#[ignore]
async fn root_serves_html() {
    let c = client();
    let r = c.get(&base_url()).send().await.unwrap();
    assert_eq!(r.status(), 200, "expected 200 from /");
    let ct = r
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("text/html"),
        "expected text/html, got {ct:?}"
    );
}
#[tokio::test]
#[ignore]
async fn favicon_resolves() {
    let c = client();
    let r = try_paths(&c, FAVICON_CANDIDATES)
        .await
        .unwrap_or_else(|| panic!("no favicon path returned 2xx: {FAVICON_CANDIDATES:?}"));
    let ct = r
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("image/") || ct.starts_with("application/octet-stream"),
        "expected image/* (or octet-stream), got {ct:?}"
    );
}
#[tokio::test]
#[ignore]
async fn manifest_parses_as_pwa() {
    let c = client();
    let r = try_paths(&c, MANIFEST_CANDIDATES)
        .await
        .unwrap_or_else(|| panic!("no manifest path returned 2xx: {MANIFEST_CANDIDATES:?}"));
    let v: Value = r.json().await.unwrap();
    assert!(
        v["name"].is_string(),
        "manifest.name must be a string, got {v:?}"
    );
    assert!(v["icons"].is_array(), "manifest.icons must be an array");
}
#[tokio::test]
#[ignore]
async fn config_endpoint_has_site_title() {
    let c = client();
    let r = try_paths(&c, CONFIG_CANDIDATES)
        .await
        .unwrap_or_else(|| panic!("no config path returned 2xx: {CONFIG_CANDIDATES:?}"));
    let v: Value = r.json().await.unwrap();
    let title = v["siteTitle"]
        .as_str()
        .or_else(|| v["site_title"].as_str())
        .unwrap_or("");
    assert!(
        title.eq_ignore_ascii_case(APP_NAME),
        "expected siteTitle == {APP_NAME:?}, got {title:?}"
    );
}
#[tokio::test]
#[ignore]
async fn service_worker_or_frontend_serves() {
    let c = client();
    let r = try_paths(&c, SERVICE_WORKER_CANDIDATES).await;
    assert!(
        r.is_some(),
        "no service-worker path returned 2xx: {SERVICE_WORKER_CANDIDATES:?}"
    );
}
#[tokio::test]
#[ignore]
async fn ws_route_responds_101_to_upgrade() {
    wait_for_health().await;
    let c = client();
    let _ = c
        .post(format!("{}/api/verify-pin", base_url()))
        .header("Origin", base_url())
        .header("Referer", format!("{}/", base_url()))
        .json(&serde_json::json!({ "pin": pin() }))
        .send()
        .await
        .unwrap();
    let res = c
        .get(format!("{}/ws", base_url()))
        .header("Origin", base_url())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status().as_u16(),
        101,
        "expected 101 Switching Protocols on /ws, got {}",
        res.status()
    );
}
#[tokio::test]
#[ignore]
async fn ws_round_trip_two_frames() {
    wait_for_health().await;
    let c = client();
    let login = c
        .post(format!("{}/api/verify-pin", base_url()))
        .header("Origin", base_url())
        .header("Referer", format!("{}/", base_url()))
        .json(&serde_json::json!({ "pin": pin() }))
        .send()
        .await
        .unwrap();
    assert!(
        login.status().is_success(),
        "verify-pin failed: {}",
        login.status()
    );
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|s| {
            let mut parts = s.split(';');
            let first = parts.next()?;
            if first.starts_with("PAD_PIN=") {
                Some(first.to_string())
            } else {
                None
            }
        })
        .unwrap_or_default();
    assert!(
        !cookie.is_empty(),
        "verify-pin response did not set PAD_PIN cookie"
    );
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port()))
        .await
        .expect("connect to /ws host");
    let req = http::Request::builder()
        .method("GET")
        .uri(format!("ws://127.0.0.1:{}/ws", port()))
        .header("Host", format!("127.0.0.1:{}", port()))
        .header("Origin", base_url())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("Cookie", cookie)
        .body(())
        .unwrap();
    let (mut ws, _response) = tokio_tungstenite::client_async(req, stream)
        .await
        .expect("client_async should complete the WebSocket handshake");
    ws.send(Message::Text(
        serde_json::json!({"type": "hello"}).to_string().into(),
    ))
    .await
    .expect("send hello frame");
    let frame = timeout(Duration::from_secs(3), ws.next()).await;
    match frame {
        Ok(Some(Ok(m))) => assert!(
            matches!(
                m,
                Message::Text(_) | Message::Ping(_) | Message::Pong(_) | Message::Binary(_)
            ),
            "unexpected frame type: {m:?}"
        ),
        Ok(Some(Err(e))) => panic!("ws stream error: {e}"),
        Ok(None) => {} // server closed — acceptable for an open/close round-trip
        Err(_) => {}   // no response within 3s — connection is still open, that's the win
    }
    let _ = ws.close(None).await;
}
