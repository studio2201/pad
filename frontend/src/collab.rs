use yew::prelude::*;
use gloo_net::websocket::futures::WebSocket;
use futures_util::{SinkExt, StreamExt};
use wasm_bindgen_futures::spawn_local;
use serde_json::json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
    export function update_peer_cursor(userId, position, color) {
        const editor = document.getElementById('editor'), container = document.getElementById('editor-container');
        if (!editor || !container) return;
        let cursor = document.getElementById('cursor-' + userId);
        if (!cursor) {
            cursor = document.createElement('div');
            cursor.id = 'cursor-' + userId;
            cursor.className = 'remote-cursor';
            Object.assign(cursor.style, { position: 'absolute', width: '2px', backgroundColor: color, pointerEvents: 'none', zIndex: '5' });
            const label = document.createElement('div');
            label.className = 'cursor-label';
            label.textContent = userId.split('_')[0];
            Object.assign(label.style, { position: 'absolute', top: '-1.2em', left: '0', backgroundColor: color, color: '#fff', fontSize: '10px', padding: '1px 3px', borderRadius: '3px', whiteSpace: 'nowrap' });
            cursor.appendChild(label);
            container.appendChild(cursor);
        }
        const textBefore = editor.value.substring(0, position);
        const tempDiv = document.createElement('div'), style = getComputedStyle(editor);
        Object.assign(tempDiv.style, { position: 'absolute', visibility: 'hidden', whiteSpace: 'pre-wrap', wordBreak: 'break-all', width: editor.clientWidth + 'px', font: style.font, lineHeight: style.lineHeight, padding: style.padding, boxSizing: 'border-box', top: '0', left: '0' });
        const span = document.createElement('span'), marker = document.createElement('span');
        span.textContent = textBefore; marker.textContent = '|';
        tempDiv.appendChild(span); tempDiv.appendChild(marker); container.appendChild(tempDiv);
        cursor.style.top = (marker.offsetTop + editor.offsetTop - editor.scrollTop) + 'px';
        cursor.style.left = (marker.offsetLeft + editor.offsetLeft) + 'px';
        cursor.style.height = style.lineHeight || '1.2em';
        cursor.style.display = 'block';
    }
    export function remove_peer_cursor(userId) {
        const cursor = document.getElementById('cursor-' + userId);
        if (cursor) cursor.remove();
    }
")]
extern "C" {
    pub fn update_peer_cursor(userId: &str, position: u32, color: &str);
    pub fn remove_peer_cursor(userId: &str);
}

pub struct Op {
    pub op_type: String,
    pub position: usize,
    pub text: String,
}

pub fn diff_strings(old: &str, new: &str) -> Vec<Op> {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let mut start = 0;
    while start < old_chars.len() && start < new_chars.len() && old_chars[start] == new_chars[start] {
        start += 1;
    }
    let mut old_end = old_chars.len();
    let mut new_end = new_chars.len();
    while old_end > start && new_end > start && old_chars[old_end - 1] == new_chars[new_end - 1] {
        old_end -= 1;
        new_end -= 1;
    }
    let mut ops = Vec::new();
    if start < old_end {
        ops.push(Op {
            op_type: "delete".to_string(),
            position: start,
            text: old_chars[start..old_end].iter().collect(),
        });
    }
    if start < new_end {
        ops.push(Op {
            op_type: "insert".to_string(),
            position: start,
            text: new_chars[start..new_end].iter().collect(),
        });
    }
    ops
}

#[hook]
pub fn use_collab_websocket(
    notepad_id: &str,
    content: UseStateHandle<String>,
    editor_ref: NodeRef,
) -> (Callback<(String, String)>, Callback<usize>) {
    let ws_sender = use_mut_ref(|| None::<futures_channel::mpsc::UnboundedSender<String>>);
    let user_id = use_state(|| format!("user_{}", chrono::Utc::now().timestamp_millis()));
    let user_color = use_state(|| {
        let colors = vec!["#3b82f6", "#10b981", "#f59e0b", "#ef4444", "#8b5cf6", "#ec4899"];
        let idx = (chrono::Utc::now().timestamp_millis() % colors.len() as i64) as usize;
        colors[idx].to_string()
    });
    
    let uid = (*user_id).clone();
    let content_clone = content.clone();
    let editor_ref_clone = editor_ref.clone();
    let ws_sender_effect = ws_sender.clone();
    
    use_effect_with(notepad_id.to_string(), move |nid| {
        let nid = nid.clone();
        let uid = uid.clone();
        let content = content_clone.clone();
        let editor_ref = editor_ref_clone.clone();
        let ws_sender_effect = ws_sender_effect.clone();
        
        let window = web_sys::window().unwrap();
        let protocol = if window.location().protocol().unwrap() == "https:" { "wss:" } else { "ws:" };
        let host = window.location().host().unwrap();
        let ws_url = format!("{}//{}/ws", protocol, host);
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
        *ws_sender_effect.borrow_mut() = Some(tx);
        
        if let Ok(ws) = WebSocket::open(&ws_url) {
            let (mut write, mut read) = ws.split();
            spawn_local(async move {
                while let Some(msg) = rx.next().await {
                    let _ = write.send(gloo_net::websocket::Message::Text(msg)).await;
                }
            });
            let init_msg = json!({
                "type": "sync_request",
                "userId": uid,
                "notepadId": nid
            }).to_string();
            let _ = ws_sender_effect.borrow().as_ref().map(|tx| tx.unbounded_send(init_msg));
            
            let uid_incoming = uid.clone();
            spawn_local(async move {
                while let Some(Ok(gloo_net::websocket::Message::Text(text))) = read.next().await {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                        let msg_type = data.get("type").and_then(|v| v.as_str());
                        let peer_id = data.get("userId").and_then(|v| v.as_str()).unwrap_or("");
                        if peer_id == uid_incoming { continue; }
                        
                        if msg_type == Some("operation") {
                            if let Some(op) = data.get("operation") {
                                let op_type = op.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let position = op.get("position").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                let text_val = op.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                
                                if let Some(textarea) = editor_ref.cast::<web_sys::HtmlTextAreaElement>() {
                                    let current_pos = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
                                    let current_end = textarea.selection_end().ok().flatten().unwrap_or(0) as usize;
                                    let mut val = textarea.value();
                                    if op_type == "insert" {
                                        val.insert_str(position, text_val);
                                    } else if op_type == "delete" {
                                        let end = std::cmp::min(val.len(), position + text_val.len());
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
                                    } else if op_type == "delete" {
                                        if position < current_pos {
                                            new_pos = std::cmp::max(position, current_pos.saturating_sub(text_val.len()));
                                            new_end = std::cmp::max(position, current_end.saturating_sub(text_val.len()));
                                        }
                                    }
                                    let _ = textarea.set_selection_range(new_pos as u32, new_end as u32);
                                }
                            }
                        } else if msg_type == Some("cursor") {
                            let color = data.get("color").and_then(|v| v.as_str()).unwrap_or("#000");
                            if let Some(position) = data.get("position").and_then(|v| v.as_u64()) {
                                update_peer_cursor(peer_id, position as u32, color);
                            }
                        } else if msg_type == Some("user_disconnected") {
                            remove_peer_cursor(peer_id);
                        }
                    }
                }
            });
        }
        let uid_cleanup = uid.clone();
        move || { remove_peer_cursor(&uid_cleanup); }
    });
    
    let uid = (*user_id).clone();
    let color = (*user_color).clone();
    let nid = notepad_id.to_string();
    let on_local_change = {
        let ws_sender = ws_sender.clone();
        let uid = uid.clone();
        let nid = nid.clone();
        Callback::from(move |(old, new): (String, String)| {
            if let Some(ref tx) = *ws_sender.borrow() {
                for op in diff_strings(&old, &new) {
                    let msg = json!({
                        "type": "operation",
                        "operation": {
                            "id": chrono::Utc::now().timestamp_millis(),
                            "type": op.op_type,
                            "position": op.position,
                            "text": op.text,
                            "userId": uid
                        },
                        "notepadId": nid,
                        "userId": uid
                    }).to_string();
                    let _ = tx.unbounded_send(msg);
                }
            }
        })
    };
    
    let on_cursor_move = {
        let ws_sender = ws_sender.clone();
        let color = color.clone();
        Callback::from(move |position: usize| {
            if let Some(ref tx) = *ws_sender.borrow() {
                let msg = json!({
                    "type": "cursor",
                    "userId": uid,
                    "color": color,
                    "position": position,
                    "notepadId": nid
                }).to_string();
                let _ = tx.unbounded_send(msg);
            }
        })
    };
    
    (on_local_change, on_cursor_move)
}
