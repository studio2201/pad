use std::net::SocketAddr;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;

use crate::state::AppState;

pub async fn handle_socket(
    ws: WebSocketUpgrade,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let origin = headers.get("origin").and_then(|h| h.to_str().ok());
    
    let allowed = if state.config.node_env == "development" || state.config.base_url == "*" {
        true
    } else if let Some(o) = origin {
        o == state.config.base_url
    } else {
        false
    };

    if !allowed {
        println!("Blocked WebSocket connection from origin: {:?}", origin);
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
        if let Message::Text(text) = msg {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = data.get("type").and_then(|v| v.as_str());
                let client_uid = data.get("userId").and_then(|v| v.as_str());
                let notepad_id = data.get("notepadId").and_then(|v| v.as_str());

                // Store user connection details on first contact
                if let Some(uid) = client_uid {
                    if user_id.is_none() {
                        user_id = Some(uid.to_string());
                        state.clients.write().await.insert(uid.to_string(), tx.clone());
                        println!("User connected via WebSocket: {}", uid);

                        let clients_map = state.clients.read().await;
                        let count = clients_map.len();
                        if count > 1 {
                            println!("Broadcasting user_connected for: {}", uid);
                            let connect_msg = serde_json::json!({
                                "type": "user_connected",
                                "userId": uid,
                                "notepadId": notepad_id,
                                "count": count
                            });
                            let msg = Message::Text(connect_msg.to_string());
                            for client_tx in clients_map.values() {
                                let _ = client_tx.send(msg.clone());
                            }
                        }
                    }
                }

                // Route message types
                if msg_type == Some("operation") && notepad_id.is_some() {
                    let nid = notepad_id.unwrap().to_string();
                    if let Some(mut op) = data.get("operation").cloned() {
                        let mut history_map = state.operations_history.write().await;
                        let history = history_map.entry(nid.clone()).or_insert_with(Vec::new);
                        let server_version = history.len();

                        if let Some(op_obj) = op.as_object_mut() {
                            op_obj.insert("serverVersion".to_string(), serde_json::json!(server_version));
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
                        let _ = tx.send(Message::Text(ack_msg.to_string()));

                        // Broadcast operation to peer connections
                        let clients_map = state.clients.read().await;
                        let broadcast_msg = serde_json::json!({
                            "type": "operation",
                            "operation": op,
                            "notepadId": nid,
                            "userId": user_id.as_ref().map(|s| s.as_str()).unwrap_or("")
                        });
                        let msg = Message::Text(broadcast_msg.to_string());
                        for (cid, client_tx) in clients_map.iter() {
                            if Some(cid) != user_id.as_ref() {
                                let _ = client_tx.send(msg.clone());
                            }
                        }
                    }
                } else if msg_type == Some("cursor") && notepad_id.is_some() {
                    let nid = notepad_id.unwrap();
                    let color = data.get("color").and_then(|v| v.as_str()).unwrap_or("");
                    let position = data.get("position").cloned().unwrap_or(serde_json::Value::Null);

                    let clients_map = state.clients.read().await;
                    let broadcast_msg = serde_json::json!({
                        "type": "cursor",
                        "userId": user_id.as_ref().map(|s| s.as_str()).unwrap_or(""),
                        "color": color,
                        "position": position,
                        "notepadId": nid
                    });
                    let msg = Message::Text(broadcast_msg.to_string());
                    for (cid, client_tx) in clients_map.iter() {
                        if Some(cid) != user_id.as_ref() {
                            let _ = client_tx.send(msg.clone());
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
                    let msg = Message::Text(broadcast_msg.to_string());
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
                    let _ = tx.send(Message::Text(sync_response.to_string()));
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

        let msg = Message::Text(disconnect_msg.to_string());
        for client_tx in clients_map.values() {
            let _ = client_tx.send(msg.clone());
        }
    }

    write_task.abort();
}
