use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::callback::Timeout;

use crate::services::ApiService;
use crate::preview::Preview;
use crate::collab::use_collab_websocket;
#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub notepad_id: String,
    pub preview_mode: String,
    pub save_interval: u64,
    pub disable_print_expand: bool,
}
#[function_component(Editor)]
pub fn editor(props: &EditorProps) -> Html {
    let content = use_state(|| "".to_string());
    let last_loaded_id = use_state(|| "".to_string());
    let debounce_timer = use_mut_ref(|| None::<Timeout>);
    let editor_ref = use_node_ref();
    let save_status = use_state(|| "saved".to_string());
    let copy_status = use_state(|| "idle".to_string());
    {
        let content = content.clone();
        let last_id = last_loaded_id.clone();
        let current_id = props.notepad_id.clone();
        
        use_effect_with(current_id.clone(), move |nid| {
            let nid = nid.clone();
            spawn_local(async move {
                if let Ok(res) = ApiService::get_notes(&nid).await {
                    content.set(res.content);
                    last_id.set(nid);
                }
            });
            || ()
        });
    }

    let (on_local_change, on_cursor_move) = use_collab_websocket(&props.notepad_id, content.clone(), editor_ref.clone());

    let on_keydown = {
        let notepad_id = props.notepad_id.clone();
        let timer_ref = debounce_timer.clone();
        let save_status = save_status.clone();
        let content = content.clone();
        
        Callback::from(move |e: KeyboardEvent| {
            let ctrl = e.ctrl_key() || e.meta_key();
            let key = e.key();
            if ctrl && key.to_lowercase() == "s" {
                e.prevent_default();
                if let Some(t) = timer_ref.borrow_mut().take() {
                    t.cancel();
                }
                let nid = notepad_id.clone();
                let save_val = (*content).clone();
                let status = save_status.clone();
                status.set("saving".to_string());
                spawn_local(async move {
                    if ApiService::save_notes(&nid, &save_val).await.is_ok() {
                        status.set("saved".to_string());
                    }
                });
            }
        })
    };

    let on_input = {
        let content = content.clone();
        let notepad_id = props.notepad_id.clone();
        let save_interval = props.save_interval;
        let timer_ref = debounce_timer.clone();
        let save_status = save_status.clone();
        let on_local_change = on_local_change.clone();
        let on_cursor_move = on_cursor_move.clone();
        
        Callback::from(move |e: InputEvent| {
            let textarea: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let val = textarea.value();
            let old_val = (*content).clone();
            on_local_change.emit((old_val, val.clone()));
            if let Some(pos) = textarea.selection_start().ok().flatten() {
                on_cursor_move.emit(pos as usize);
            }
            content.set(val.clone());
            save_status.set("unsaved".to_string());
            
            if let Some(t) = timer_ref.borrow_mut().take() {
                t.cancel();
            }
            
            if save_interval > 0 {
                let nid = notepad_id.clone();
                let save_val = val.clone();
                let status = save_status.clone();
                let new_timer = Timeout::new(save_interval as u32, move || {
                    status.set("saving".to_string());
                    spawn_local(async move {
                        if ApiService::save_notes(&nid, &save_val).await.is_ok() {
                            status.set("saved".to_string());
                        }
                    });
                });
                *timer_ref.borrow_mut() = Some(new_timer);
            }
        })
    };

    let on_click = {
        let r = editor_ref.clone(); let m = on_cursor_move.clone();
        Callback::from(move |_: MouseEvent| {
            let _ = r.cast::<web_sys::HtmlTextAreaElement>().map(|t| t.selection_start().ok().flatten().map(|p| m.emit(p as usize)));
        })
    };
    let on_keyup = {
        let r = editor_ref.clone(); let m = on_cursor_move.clone();
        Callback::from(move |_: KeyboardEvent| {
            let _ = r.cast::<web_sys::HtmlTextAreaElement>().map(|t| t.selection_start().ok().flatten().map(|p| m.emit(p as usize)));
        })
    };
    let on_scroll = {
        let r = editor_ref.clone(); let m = on_cursor_move.clone();
        Callback::from(move |_: Event| {
            let _ = r.cast::<web_sys::HtmlTextAreaElement>().map(|t| t.selection_start().ok().flatten().map(|p| m.emit(p as usize)));
        })
    };

    let on_blur = {
        let notepad_id = props.notepad_id.clone();
        let content = content.clone();
        let timer_ref = debounce_timer.clone();
        let save_status = save_status.clone();
        
        Callback::from(move |_| {
            if let Some(t) = timer_ref.borrow_mut().take() {
                t.cancel();
                let nid = notepad_id.clone();
                let save_val = (*content).clone();
                let status = save_status.clone();
                status.set("saving".to_string());
                spawn_local(async move {
                    if ApiService::save_notes(&nid, &save_val).await.is_ok() {
                        status.set("saved".to_string());
                    }
                });
            }
        })
    };

    let on_copy = {
        let content = content.clone();
        let copy_status = copy_status.clone();
        Callback::from(move |_| {
            let content_val = (*content).clone();
            let copy_status = copy_status.clone();
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let _ = clipboard.write_text(&content_val);
                copy_status.set("copied".to_string());
                let copy_status_clone = copy_status.clone();
                let _ = gloo_timers::callback::Timeout::new(2000, move || {
                    copy_status_clone.set("idle".to_string());
                }).forget();
            }
        })
    };

    let on_export = {
        let content = content.clone();
        let notepad_id = props.notepad_id.clone();
        Callback::from(move |_| {
            let content_val = (*content).clone();
            let filename = format!("{}.md", notepad_id);
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    let encoded = percent_encoding::utf8_percent_encode(&content_val, percent_encoding::NON_ALPHANUMERIC).to_string();
                    let href = format!("data:text/markdown;charset=utf-8,{}", encoded);
                    if let Ok(a) = document.create_element("a") {
                        let a: web_sys::HtmlElement = a.unchecked_into();
                        let _ = a.set_attribute("href", &href);
                        let _ = a.set_attribute("download", &filename);
                        a.click();
                    }
                }
            }
        })
    };

    let (copy_icon, copy_text_style, copy_text) = if *copy_status == "copied" {
        (html! { <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px; color: #10b981;"><polyline points="20 6 9 17 4 12"></polyline></svg> }, Some("color: #10b981;"), "Copied!")
    } else {
        (html! { <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px;"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg> }, None, "Copy")
    };

    let show_editor = props.preview_mode != "preview-only";
    let show_preview = props.preview_mode != "off";

    html! {
        <div id="editor-preview-wrapper" class="editor-preview-wrapper">
            if show_editor {
                <div id="editor-container" class={classes!("editor-container", if props.preview_mode == "split" { Some("split-view") } else { None })}>
                    <textarea 
                        id="editor" 
                        ref={editor_ref}
                        placeholder="Start typing your notes here..." 
                        spellcheck="true" 
                        value={(*content).clone()}
                        oninput={on_input}
                        onblur={on_blur}
                        onkeydown={on_keydown}
                        onclick={on_click}
                        onkeyup={on_keyup}
                        onscroll={on_scroll}
                        autofocus=true
                    />
                    <div class="editor-actions">
                        <button class="action-button copy-button" onclick={on_copy} data-tooltip="Copy Markdown">
                            {copy_icon}
                            <span style={copy_text_style}>{copy_text}</span>
                        </button>
                        <button class="action-button export-button" onclick={on_export} data-tooltip="Export Markdown">
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 4px;"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
                            <span>{"Export"}</span>
                        </button>
                    </div>
                    <div class={classes!("save-status", (*save_status).clone())}>
                        {
                            match save_status.as_str() {
                                "unsaved" => html! { <>{"● "}{"Unsaved changes"}</> },
                                "saving" => html! { <>{"◌ "}{"Saving..."}</> },
                                _ => html! { <>{"✓ "}{"Saved"}</> },
                            }
                        }
                    </div>
                </div>
            }
            
            if show_preview {
                <Preview 
                    content={(*content).clone()} 
                    is_visible={show_preview} 
                />
            }
        </div>
    }
}
