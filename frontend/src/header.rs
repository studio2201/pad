use yew::prelude::*;
use crate::types::Notepad;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::services::StorageService;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub active_notepad_id: String,
    pub notepads: Vec<Notepad>,
    pub on_notepad_select: Callback<Event>,
    pub on_new_notepad: Callback<MouseEvent>,
    pub on_rename: Callback<MouseEvent>,
    pub on_delete: Callback<MouseEvent>,
    pub preview_mode: String,
    pub on_preview_toggle: Callback<MouseEvent>,
    pub app_version: String,
    pub on_search_open: Callback<MouseEvent>,
    pub on_settings_open: Callback<MouseEvent>,
    pub on_shortcuts_open: Callback<MouseEvent>,
    pub toggle_theme: Callback<MouseEvent>,
    pub on_logout: Callback<MouseEvent>,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let peer_count = use_state(|| 1u32);

    {
        let peer_count = peer_count.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let callback = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
                if let Ok(detail) = js_sys::Reflect::get(&e, &JsValue::from_str("detail")) {
                    if let Some(val) = detail.as_f64() {
                        peer_count.set(val as u32);
                    }
                }
            });
            let _ = window.add_event_listener_with_callback("rustpad:peer_count", callback.as_ref().unchecked_ref());
            move || {
                let _ = window.remove_event_listener_with_callback("rustpad:peer_count", callback.as_ref().unchecked_ref());
            }
        });
    }

    let current_theme = StorageService::get_theme();
    let theme_toggle_icon = match current_theme.as_str() {
        "dark" => html! {
            <svg id="moon-icon" class="moon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z" /></svg>
        },
        "nord" => html! {
            <svg id="droplet-icon" class="droplet" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22a7 7 0 0 0 7-7c0-4.3-7-13-7-13S5 10.7 5 15a7 7 0 0 0 7 7z"/></svg>
        },
        "dracula" => html! {
            <svg id="sparkles-icon" class="sparkles" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275Z"/><path d="m5 3 1 2.5L8.5 6 6 7 5 9.5 4 7 1.5 6 4 5Z"/><path d="m19 17 1 2.5 2.5.5-2.5 1-1 2.5-1-2.5-2.5-1 2.5-1Z"/></svg>
        },
        "sepia" => html! {
            <svg id="coffee-icon" class="coffee" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 8h1a4 4 0 1 1 0 8h-1"/><path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z"/><line x1="6" y1="2" x2="6" y2="4"/><line x1="10" y1="2" x2="10" y2="4"/><line x1="14" y1="2" x2="14" y2="4"/></svg>
        },
        _ => html! {
            <svg id="sun-icon" class="sun" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2" /><path d="M12 20v2" /><path d="M4.93 4.93l1.41 1.41" /><path d="M17.66 17.66l1.41 1.41" /><path d="M2 12h2" /><path d="M20 12h2" /><path d="M6.34 17.66l-1.41 1.41" /><path d="M19.07 4.93l-1.41 1.41" /></svg>
        },
    };

    let peer_count_val = *peer_count;
    let peers_badge = if peer_count_val > 1 {
        html! {
            <span class="active-peers-badge" style="display: inline-flex; align-items: center; gap: 6px; font-size: 0.75rem; background: rgba(16, 185, 129, 0.15); color: #10b981; padding: 4px 8px; border-radius: 9999px; font-weight: 600; margin-left: 10px; border: 1px solid rgba(16, 185, 129, 0.3);">
                <span class="pulse-dot" style="width: 6px; height: 6px; background-color: #10b981; border-radius: 50%; display: inline-block;"></span>
                {format!("{} online", peer_count_val)}
            </span>
        }
    } else {
        html! {}
    };

    html! {
        <header>
            <div class="notepad-controls">
                <div class="select-wrapper">
                    <button id="new-notepad" class="icon-button" onclick={props.on_new_notepad.clone()} aria-label="Create new notepad">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                    </button>
                    <select id="notepad-selector" onchange={props.on_notepad_select.clone()} value={props.active_notepad_id.clone()}>
                        {
                            for props.notepads.iter().map(|n| {
                                html! { <option value={n.id.clone()}>{&n.name}</option> }
                            })
                        }
                    </select>
                </div>
                <div class="notepad-controls-wrapper">
                    <button id="rename-notepad" class="icon-button" onclick={props.on_rename.clone()}>
                        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path></svg>
                    </button>
                    <button id="delete-notepad" class="icon-button" onclick={props.on_delete.clone()}>
                        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18"></path><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path></svg>
                    </button>
                    <button id="preview-markdown" class="icon-button" onclick={props.on_preview_toggle.clone()}>
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4"><path d="M3 5m0 2a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v10a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2z" /><path d="M7 15v-6l2 2l2 -2v6" /><path d="M14 13l2 2l2 -2m-2 2v-6" /></svg>
                    </button>
                </div>
            </div>
            <div id="header-title" data-tooltip={format!("Version: {}", props.app_version)} style="display: flex; align-items: center;">
                <h1 style="font-size: 1.5rem; margin: 0;">{"RustPad"}</h1>
                {peers_badge}
            </div>
            <div class="header-right">
                <button id="shortcuts-button" class="icon-button" onclick={props.on_shortcuts_open.clone()} data-tooltip="Keyboard Shortcuts Help">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                </button>
                <button id="search-open" class="icon-button" onclick={props.on_search_open.clone()} data-tooltip="Search">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 10m-7 0a7 7 0 1 0 14 0a7 7 0 1 0 -14 0" /><path d="M21 21l-6 -6" /></svg>
                </button>
                <button id="settings-button" class="icon-button" onclick={props.on_settings_open.clone()} data-tooltip="Settings">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 6m-2 0a2 2 0 1 0 4 0a2 2 0 1 0 -4 0" /><path d="M4 6l8 0" /><path d="M16 6l4 0" /><path d="M8 12m-2 0a2 2 0 1 0 4 0a2 2 0 1 0 -4 0" /><path d="M4 12l2 0" /><path d="M10 12l10 0" /><path d="M17 18m-2 0a2 2 0 1 0 4 0a2 2 0 1 0 -4 0" /><path d="M4 18l11 0" /><path d="M19 18l1 0" /></svg>
                </button>
                <button id="theme-toggle" class="icon-button" onclick={props.toggle_theme.clone()}>
                    {theme_toggle_icon}
                </button>
                <button id="logout-button" class="icon-button" onclick={props.on_logout.clone()} data-tooltip="Log Out">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><polyline points="16 17 21 12 16 7" /><line x1="21" y1="12" x2="9" y2="12" /></svg>
                </button>
            </div>
        </header>
    }
}
