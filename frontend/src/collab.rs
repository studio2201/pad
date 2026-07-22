use crate::collab_utils::{
    diff_strings, dispatch_peer_count, remove_peer_cursor, update_peer_cursor,
};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[hook]
pub fn use_collab_websocket(
    notepad_id: &str,
    content: UseStateHandle<String>,
    editor_ref: NodeRef,
) -> (Callback<(String, String)>, Callback<usize>) {
    let ws_sender = use_mut_ref(|| None::<futures_channel::mpsc::UnboundedSender<String>>);
    let user_id = use_state(|| format!("user_{}", js_sys::Date::now() as i64));
    let user_color = use_state(|| {
        let colors = [
            "#3b82f6", "#10b981", "#f59e0b", "#ef4444", "#8b5cf6", "#ec4899",
        ];
        let idx = ((js_sys::Date::now() as i64) % colors.len() as i64) as usize;
        colors[idx].to_string()
    });
    let offline_queue = use_mut_ref(Vec::<String>::new);
    let cancelled = use_mut_ref(|| false);

    let uid = (*user_id).clone();
    let color = (*user_color).clone();
    let content_clone = content.clone();
    let editor_ref_clone = editor_ref.clone();
    let ws_sender_effect = ws_sender.clone();
    let offline_queue_effect = offline_queue.clone();
    let cancelled_effect = cancelled.clone();

    let uid_for_effect = uid.clone();
    let nid = notepad_id.to_string();
    use_effect_with(notepad_id.to_string(), move |nid| {
        let nid = nid.clone();
        let uid = uid_for_effect.clone();
        let content = content_clone.clone();
        let editor_ref = editor_ref_clone.clone();
        let ws_sender_effect = ws_sender_effect.clone();
        let offline_queue_effect = offline_queue_effect.clone();
        let cancelled = cancelled_effect.clone();
        *cancelled.borrow_mut() = false;

        let uid_spawn = uid.clone();
        spawn_local(async move {
            let mut backoff = 1000;
            loop {
                if *cancelled.borrow() {
                    break;
                }
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => break,
                };
                let protocol = match window.location().protocol() {
                    Ok(p) if p == "https:" => "wss:",
                    _ => "ws:",
                };
                let host = match window.location().host() {
                    Ok(h) => h,
                    Err(_) => break,
                };
                let ws_url = format!("{}//{}/ws", protocol, host);

                if let Ok(ws) = WebSocket::open(&ws_url) {
                    backoff = 1000;
                    let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
                    *ws_sender_effect.borrow_mut() = Some(tx);
                    let (mut write, mut read) = ws.split();

                    let init_msg = json!({
                        "type": "sync_request",
                        "userId": uid_spawn,
                        "notepadId": nid
                    })
                    .to_string();
                    let _ = ws_sender_effect
                        .borrow()
                        .as_ref()
                        .map(|tx| tx.unbounded_send(init_msg));

                    {
                        let mut queue = offline_queue_effect.borrow_mut();
                        for msg in queue.drain(..) {
                            let _ = ws_sender_effect
                                .borrow()
                                .as_ref()
                                .map(|tx| tx.unbounded_send(msg));
                        }
                    }

                    spawn_local(async move {
                        while let Some(msg) = rx.next().await {
                            let _ = write.send(gloo_net::websocket::Message::Text(msg)).await;
                        }
                    });

                    let uid_incoming = uid_spawn.clone();
                    while let Some(Ok(gloo_net::websocket::Message::Text(text))) = read.next().await
                    {
                        if *cancelled.borrow() {
                            break;
                        }
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                            let msg_type = data.get("type").and_then(|v| v.as_str());
                            let peer_id = data.get("userId").and_then(|v| v.as_str()).unwrap_or("");
                            if (msg_type == Some("user_connected")
                                || msg_type == Some("user_disconnected"))
                                && let Some(count) = data.get("count").and_then(|v| v.as_u64())
                            {
                                dispatch_peer_count(count as u32);
                            }
                            if peer_id == uid_incoming {
                                continue;
                            }
                            if msg_type == Some("operation") {
                                if let Some(op) = data.get("operation") {
                                    let op_type =
                                        op.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                    let position =
                                        op.get("position").and_then(|v| v.as_u64()).unwrap_or(0)
                                            as usize;
                                    let text_val =
                                        op.get("text").and_then(|v| v.as_str()).unwrap_or("");

                                    if let Some(textarea) =
                                        editor_ref.cast::<web_sys::HtmlTextAreaElement>()
                                    {
                                        let current_pos =
                                            textarea.selection_start().ok().flatten().unwrap_or(0)
                                                as usize;
                                        let current_end =
                                            textarea.selection_end().ok().flatten().unwrap_or(0)
                                                as usize;
                                        let mut val = textarea.value();
                                        if op_type == "insert" {
                                            val.insert_str(position, text_val);
                                        } else if op_type == "delete" {
                                            let end =
                                                std::cmp::min(val.len(), position + text_val.len());
                                            val.drain(position..end);
                                        }
                                        textarea.set_value(&val);
                                        content.set(val);

                                        let mut new_pos = current_pos;
                                        let mut new_end = current_end;
                                        if op_type == "insert" {
                                            if position < current_pos {
                                                new_pos += text_val.len();
                                                new_end += text_val.len();
                                            }
                                        } else if op_type == "delete" && position < current_pos {
                                            new_pos = std::cmp::max(
                                                position,
                                                current_pos.saturating_sub(text_val.len()),
                                            );
                                            new_end = std::cmp::max(
                                                position,
                                                current_end.saturating_sub(text_val.len()),
                                            );
                                        }
                                        let _ = textarea
                                            .set_selection_range(new_pos as u32, new_end as u32);
                                    }
                                }
                            } else if msg_type == Some("cursor") {
                                let color =
                                    data.get("color").and_then(|v| v.as_str()).unwrap_or("#000");
                                if let Some(position) =
                                    data.get("position").and_then(|v| v.as_u64())
                                {
                                    update_peer_cursor(peer_id, position as u32, color);
                                }
                            } else if msg_type == Some("user_disconnected") {
                                remove_peer_cursor(peer_id);
                            }
                        }
                    }
                }
                *ws_sender_effect.borrow_mut() = None;
                dispatch_peer_count(1);

                if *cancelled.borrow() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(backoff).await;
                backoff = std::cmp::min(16000, backoff * 2);
            }
        });

        let cancelled_cleanup = cancelled_effect.clone();
        let uid_cleanup = uid.clone();
        move || {
            *cancelled_cleanup.borrow_mut() = true;
            remove_peer_cursor(&uid_cleanup);
            dispatch_peer_count(1);
        }
    });

    let on_local_change = {
        let (ws_sender, offline_queue, uid, nid) = (
            ws_sender.clone(),
            offline_queue.clone(),
            uid.clone(),
            nid.clone(),
        );
        Callback::from(move |(old, new): (String, String)| {
            for op in diff_strings(&old, &new) {
                let msg = json!({"type": "operation", "operation": {"id": js_sys::Date::now() as i64, "type": op.op_type, "position": op.position, "text": op.text, "userId": uid}, "notepadId": nid, "userId": uid}).to_string();
                if let Some(ref tx) = *ws_sender.borrow() {
                    let _ = tx.unbounded_send(msg);
                } else {
                    offline_queue.borrow_mut().push(msg);
                }
            }
        })
    };

    let on_cursor_move = {
        let ws_sender = ws_sender.clone();
        Callback::from(move |position: usize| {
            if let Some(ref tx) = *ws_sender.borrow() {
                let msg = json!({"type": "cursor", "userId": uid, "color": color, "position": position, "notepadId": nid}).to_string();
                let _ = tx.unbounded_send(msg);
            }
        })
    };

    (on_local_change, on_cursor_move)
}
