//! WebSocket sync for collaborative notepad editing.
//!
//! FIXME: This is a **broadcast-only** implementation. Concurrent typing by
//! two users at different positions in the document will result in
//! divergent textareas; the only shared notion of state is the on-disk
//! file (which is last-writer-wins on save). True collaborative editing
//! requires either:
//!   - Operational Transformation (e.g. the `ot` crate)
//!   - CRDTs (e.g. `y-crdt`, `automerge`)
//!
//! Tracked as a separate issue. The current implementation is suitable
//! for single-writer or low-conflict multi-writer scenarios only.

use axum::{
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::state::AppState;

/// Decide whether a WebSocket upgrade request from `origin_header` is allowed.
///
/// The rules:
///
/// - In `development` (`NODE_ENV=development`), all origins are allowed.
///   This is the documented developer convenience for local hacking.
/// - In any other environment, the request's `Origin` header **must** match
///   the configured `base_url` (modulo trailing slash). Requests with no
///   `Origin` header are rejected (browsers always send one for cross-origin
///   upgrades; non-browser clients can set it).
///
/// The previous `base_url == "*"` shortcut that disabled the check in
/// production has been removed: a misconfigured `BASE_URL=*` (the `.env.example`
/// default for many setups) used to silently disable CSRF protection on the
/// WebSocket. Now such configurations will reject all connections until the
/// operator sets `BASE_URL` to the public URL or runs with `NODE_ENV=development`.
pub fn is_origin_allowed(
    origin_header: Option<&str>,
    config_base_url: &str,
    node_env: &str,
) -> bool {
    if node_env == "development" {
        return true;
    }
    let Some(origin) = origin_header else {
        return false;
    };
    origin.trim_end_matches('/') == config_base_url.trim_end_matches('/')
}

pub async fn handle_socket(
    ws: WebSocketUpgrade,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let origin = headers.get("origin").and_then(|h| h.to_str().ok());

    let allowed = is_origin_allowed(
        origin,
        &state.config.server.base_url,
        &state.config.node_env,
    );

    if !allowed {
        tracing::warn!(
            target: "ws",
            "Blocked WebSocket upgrade from origin {:?} (base_url={}, env={})",
            origin,
            state.config.server.base_url,
            state.config.node_env,
        );
        return (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    ws.on_upgrade(move |socket| ws_handler(socket, state))
}

async fn ws_handler(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Forward outbound messages from channel to WebSocket sender
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut user_id: Option<String> = None;

    // WebSocket read loop
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg
            && let Ok(data) = serde_json::from_str::<serde_json::Value>(&text)
        {
            let msg_type = data.get("type").and_then(|v| v.as_str());
            let client_uid = data.get("userId").and_then(|v| v.as_str());
            let notepad_id = data.get("notepadId").and_then(|v| v.as_str());

            // Store user connection details on first contact
            if let Some(uid) = client_uid
                && user_id.is_none()
            {
                user_id = Some(uid.to_string());
                state
                    .clients
                    .write()
                    .await
                    .insert(uid.to_string(), tx.clone());
                println!("User connected via WebSocket: {}", uid);

                let clients_map = state.clients.read().await;
                let count = clients_map.len();
                println!(
                    "Broadcasting user_connected for: {}, total count: {}",
                    uid, count
                );
                let connect_msg = serde_json::json!({
                    "type": "user_connected",
                    "userId": uid,
                    "notepadId": notepad_id,
                    "count": count
                });
                let msg = Message::Text(connect_msg.to_string().into());
                for client_tx in clients_map.values() {
                    let _ = client_tx.send(msg.clone());
                }
            }

            // Route message types
            if msg_type == Some("operation") {
                if let Some(nid) = notepad_id {
                    let nid = nid.to_string();
                    if let Some(mut op) = data.get("operation").cloned() {
                        let mut history_map = state.operations_history.write().await;
                        let history = history_map.entry(nid.clone()).or_insert_with(Vec::new);
                        let server_version = history.len();

                        if let Some(op_obj) = op.as_object_mut() {
                            op_obj.insert(
                                "serverVersion".to_string(),
                                serde_json::json!(server_version),
                            );
                        }
                        history.push(op.clone());

                        // Drain history if exceeds 1000 items
                        if history.len() > 1000 {
                            history.drain(0..history.len() - 1000);
                        }

                        // Send ACK message to the client
                        let op_id = op.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let ack_msg = serde_json::json!({
                            "type": "ack",
                            "operationId": op_id,
                            "serverVersion": server_version
                        });
                        let _ = tx.send(Message::Text(ack_msg.to_string().into()));

                        // Broadcast operation to peer connections
                        let clients_map = state.clients.read().await;
                        let broadcast_msg = serde_json::json!({
                            "type": "operation",
                            "operation": op,
                            "notepadId": nid,
                            "userId": user_id.as_deref().unwrap_or("")
                        });
                        let msg = Message::Text(broadcast_msg.to_string().into());
                        for (cid, client_tx) in clients_map.iter() {
                            if Some(cid) != user_id.as_ref() {
                                let _ = client_tx.send(msg.clone());
                            }
                        }
                    }
                }
            } else if msg_type == Some("cursor") {
                if let Some(nid) = notepad_id {
                    let color = data.get("color").and_then(|v| v.as_str()).unwrap_or("");
                    let position = data
                        .get("position")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let clients_map = state.clients.read().await;
                    let broadcast_msg = serde_json::json!({
                        "type": "cursor",
                        "userId": user_id.as_deref().unwrap_or(""),
                        "color": color,
                        "position": position,
                        "notepadId": nid
                    });
                    let msg = Message::Text(broadcast_msg.to_string().into());
                    for (cid, client_tx) in clients_map.iter() {
                        if Some(cid) != user_id.as_ref() {
                            let _ = client_tx.send(msg.clone());
                        }
                    }
                }
            } else if msg_type == Some("notepad_rename") {
                let nid = data.get("notepadId").and_then(|v| v.as_str()).unwrap_or("");
                let new_name = data.get("newName").and_then(|v| v.as_str()).unwrap_or("");

                let clients_map = state.clients.read().await;
                let broadcast_msg = serde_json::json!({
                    "type": "notepad_rename",
                    "notepadId": nid,
                    "newName": new_name
                });
                let msg = Message::Text(broadcast_msg.to_string().into());
                for (cid, client_tx) in clients_map.iter() {
                    if Some(cid) != user_id.as_ref() {
                        let _ = client_tx.send(msg.clone());
                    }
                }
            } else if msg_type == Some("sync_request") && notepad_id.is_some() {
                let nid = notepad_id.unwrap();
                let history_map = state.operations_history.read().await;
                let history = history_map.get(nid).cloned().unwrap_or_default();

                let sync_response = serde_json::json!({
                    "type": "sync_response",
                    "operations": history,
                    "notepadId": nid
                });
                let _ = tx.send(Message::Text(sync_response.to_string().into()));
            } else {
                // Fallback to broadcasting other messages directly
                let clients_map = state.clients.read().await;
                let msg = Message::Text(text.clone());
                for (cid, client_tx) in clients_map.iter() {
                    if Some(cid) != user_id.as_ref() {
                        let _ = client_tx.send(msg.clone());
                    }
                }
            }
        }
    }

    // Cleanup user mappings upon disconnect
    if let Some(ref uid) = user_id {
        state.clients.write().await.remove(uid);
        println!("User disconnected via WebSocket: {}", uid);

        let clients_map = state.clients.read().await;
        let count = clients_map.len();
        let disconnect_msg = serde_json::json!({
            "type": "user_disconnected",
            "userId": uid,
            "count": count
        });

        let msg = Message::Text(disconnect_msg.to_string().into());
        for client_tx in clients_map.values() {
            let _ = client_tx.send(msg.clone());
        }
    }

    write_task.abort();
}
