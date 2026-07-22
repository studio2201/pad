use axum::{
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use super::origin::is_origin_allowed;
use crate::routes::auth::is_authenticated;
use crate::state::AppState;

pub async fn handle_socket(
    ws: WebSocketUpgrade,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    // REST routes gate on PIN/session; the collab socket must do the same or
    // an unauthenticated client can inject OT ops into an otherwise locked pad.
    if !is_authenticated(&jar, &state, &headers).await {
        tracing::warn!(target: "ws", "Blocked unauthenticated WebSocket upgrade");
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

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
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
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

            // Store user connection details on first contact.
            // Never overwrite an existing client channel for the same userId —
            // that would let a late joiner hijack another peer's send path.
            if let Some(uid) = client_uid
                && user_id.is_none()
            {
                let assigned = {
                    let mut clients = state.clients.write().await;
                    let mut assigned = uid.to_string();
                    if clients.contains_key(&assigned) {
                        assigned = format!("{}-{}", uid, &uuid_v4_lite());
                    }
                    clients.insert(assigned.clone(), tx.clone());
                    assigned
                };
                user_id = Some(assigned.clone());
                tracing::info!(target: "ws", "User connected via WebSocket: {assigned}");

                let clients_map = state.clients.read().await;
                let count = clients_map.len();
                let connect_msg = serde_json::json!({
                    "type": "user_connected",
                    "userId": assigned,
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
            } else if msg_type == Some("sync_request")
                && let Some(nid) = notepad_id
            {
                let history_map = state.operations_history.read().await;
                let history = history_map.get(nid).cloned().unwrap_or_default();

                let sync_response = serde_json::json!({
                    "type": "sync_response",
                    "operations": history,
                    "notepadId": nid
                });
                let _ = tx.send(Message::Text(sync_response.to_string().into()));
            }
            // Unknown message types are ignored (no open-ended broadcast).
        }
    }

    // Cleanup user mappings upon disconnect
    if let Some(ref uid) = user_id {
        state.clients.write().await.remove(uid);
        tracing::info!(target: "ws", "User disconnected via WebSocket: {uid}");

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

/// Lightweight random id for disambiguating colliding client userIds.
/// Avoids pulling a full UUID crate just for channel map keys.
fn uuid_v4_lite() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mix = nanos.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    format!("{mix:016x}")
}
