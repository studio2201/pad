use crate::{api::ApiService, collab::use_collab_websocket};
use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub notepad_id: String,
    pub save_interval: u64,
    pub disable_print_expand: bool,
    pub on_status: Callback<Option<(String, String)>>,
    pub on_content_empty: Callback<bool>,
}

fn save_notepad(id: String, content: String, status: UseStateHandle<String>) {
    status.set("saving".to_string());
    spawn_local(async move {
        if ApiService::save_notes(&id, &content).await.is_ok() {
            status.set("saved".to_string());
        }
    });
}

#[function_component(Editor)]
pub fn editor(props: &EditorProps) -> Html {
    let content = use_state(|| "".to_string());
    let last_loaded_id = use_state(|| "".to_string());
    let debounce_timer = use_mut_ref(|| None::<Timeout>);
    let editor_ref = use_node_ref();
    let save_status = use_state(|| "saved".to_string());
    let locale = use_context::<crate::i18n::LocaleContext>().unwrap();

    {
        let on_content_empty = props.on_content_empty.clone();
        let content_str = (*content).clone();
        use_effect_with(content_str, move |c| {
            on_content_empty.emit(c.trim().is_empty());
            || ()
        });
    }

    {
        let on_status = props.on_status.clone();
        let save_status = save_status.clone();
        let locale = locale.clone();
        use_effect_with(save_status.clone(), move |save| {
            let save = save.clone();
            match save.as_str() {
                "unsaved" => on_status.emit(Some((
                    format!("● {}", locale.t("unsaved_changes")),
                    "error".to_string(),
                ))),
                "saving" => on_status.emit(Some((
                    format!("◌ {}", locale.t("saving")),
                    "info".to_string(),
                ))),
                _ => on_status.emit(Some((
                    format!("✓ {}", locale.t("saved")),
                    "success".to_string(),
                ))),
            }
            || ()
        });
    }

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

    let (on_local_change, on_cursor_move) =
        use_collab_websocket(&props.notepad_id, content.clone(), editor_ref.clone());

    let on_keydown = {
        let (nid, timer, status, content) = (
            props.notepad_id.clone(),
            debounce_timer.clone(),
            save_status.clone(),
            content.clone(),
        );
        Callback::from(move |e: KeyboardEvent| {
            if (e.ctrl_key() || e.meta_key()) && e.key().to_lowercase() == "s" {
                e.prevent_default();
                if let Some(t) = timer.borrow_mut().take() {
                    t.cancel();
                }
                save_notepad(nid.clone(), (*content).clone(), status.clone());
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
                    save_notepad(nid, save_val, status);
                });
                *timer_ref.borrow_mut() = Some(new_timer);
            }
        })
    };

    let update_cursor_pos = {
        let r = editor_ref.clone();
        let m = on_cursor_move.clone();
        move || {
            let _ = r.cast::<web_sys::HtmlTextAreaElement>().map(|t| {
                t.selection_start()
                    .ok()
                    .flatten()
                    .map(|p| m.emit(p as usize))
            });
        }
    };
    let on_click = {
        let u = update_cursor_pos.clone();
        Callback::from(move |_: MouseEvent| u())
    };
    let on_keyup = {
        let u = update_cursor_pos.clone();
        Callback::from(move |_: KeyboardEvent| u())
    };
    let on_scroll = {
        let u = update_cursor_pos;
        Callback::from(move |_: Event| u())
    };

    let on_blur = {
        let notepad_id = props.notepad_id.clone();
        let content = content.clone();
        let timer_ref = debounce_timer.clone();
        let save_status = save_status.clone();
        Callback::from(move |_| {
            if let Some(t) = timer_ref.borrow_mut().take() {
                t.cancel();
                save_notepad(notepad_id.clone(), (*content).clone(), save_status.clone());
            }
        })
    };

    html! {
        <div id="editor-preview-wrapper" class="editor-preview-wrapper">
            <div id="editor-container" class="editor-container" style="border-top-left-radius: 8px; border-top-right-radius: 8px;">
                <textarea
                    id="editor"
                    ref={editor_ref}
                    placeholder={locale.t("placeholder")}
                    spellcheck="true"
                    value={(*content).clone()}
                    oninput={on_input}
                    onblur={on_blur}
                    onkeydown={on_keydown}
                    onclick={on_click}
                    onkeyup={on_keyup}
                    onscroll={on_scroll}
                    autofocus=true
                    style="border-top-left-radius: 8px; border-top-right-radius: 8px;"
                />
            </div>
        </div>
    }
}
