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
        tempDiv.remove();
    }
    export function remove_peer_cursor(userId) {
        const cursor = document.getElementById('cursor-' + userId);
        if (cursor) cursor.remove();
    }
    export function dispatch_peer_count(count) {
        window.dispatchEvent(new CustomEvent('log:peer_count', { detail: count }));
    }
")]
extern "C" {
    pub fn update_peer_cursor(userId: &str, position: u32, color: &str);
    pub fn remove_peer_cursor(userId: &str);
    pub fn dispatch_peer_count(count: u32);
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
    while start < old_chars.len() && start < new_chars.len() && old_chars[start] == new_chars[start]
    {
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
