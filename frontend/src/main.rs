mod types;
mod services;
mod login;
mod settings;
mod preview;
mod search;
mod editor;
mod modals;
mod shortcuts;
mod collab;
mod collab_utils;
mod header;
mod toolbar;
use yew::prelude::*;
use shortcuts::register_keyboard_shortcuts;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;
use types::Notepad;
use services::{ApiService, StorageService};
use login::Login;
use settings::SettingsModal;
use search::SearchModal;
use editor::Editor;
use modals::{RenameModal, DeleteModal, ShortcutsModal};
use header::Header;

#[function_component(App)]
pub fn app() -> Html {
    let authenticated = use_state(|| false);
    let notepads = use_state(|| Vec::<Notepad>::new());
    let active_notepad_id = use_state(|| "default".to_string());
    let settings = use_state(StorageService::get_settings);
    let preview_mode = use_state(|| "off".to_string());
    let search_open = use_state(|| false);
    let settings_open = use_state(|| false);
    let rename_open = use_state(|| false);
    let delete_open = use_state(|| false);
    let shortcuts_open = use_state(|| false);
    let app_version = use_state(|| "1.0.5".to_string());
    let theme = use_state(StorageService::get_theme);

    {
        let authenticated = authenticated.clone();
        let notepads = notepads.clone();
        let active_id = active_notepad_id.clone();
        let preview = preview_mode.clone();
        let s = settings.clone();
        let version = app_version.clone();
        
        use_effect_with((*authenticated).clone(), move |&auth| {
            if auth {
                spawn_local(async move {
                    if let Ok(config) = ApiService::get_config().await {
                        version.set(config.version);
                    }
                    if let Ok(res) = ApiService::get_notepads().await {
                        notepads.set(res.notepads_list);
                        active_id.set(res.note_history);
                    }
                    let cur_s = StorageService::get_settings();
                    preview.set(cur_s.default_markdown_preview_mode.clone());
                    s.set(cur_s);
                });
            }
            || ()
        });
    }

    let on_new_notepad = {
        let notepads = notepads.clone();
        let active_id = active_notepad_id.clone();
        Callback::from(move |_| {
            let (n, a) = (notepads.clone(), active_id.clone());
            spawn_local(async move {
                if let Ok(note) = ApiService::create_notepad().await {
                    a.set(note.id);
                    if let Ok(res) = ApiService::get_notepads().await { n.set(res.notepads_list); }
                }
            });
        })
    };

    register_keyboard_shortcuts(
        authenticated.clone(),
        search_open.clone(),
        shortcuts_open.clone(),
        preview_mode.clone(),
        on_new_notepad.clone(),
    );

    if !*authenticated {
        return html! {
            <Login on_login_success={Callback::from(move |_| authenticated.set(true))} />
        };
    }

    let on_notepad_select = {
        let active_id = active_notepad_id.clone();
        Callback::from(move |e: Event| active_id.set(e.target_unchecked_into::<web_sys::HtmlSelectElement>().value()))
    };

    let on_rename_confirm = {
        let nid = (*active_notepad_id).clone();
        let rename_open = rename_open.clone();
        let notepads = notepads.clone();
        Callback::from(move |new_name: String| {
            let (nid, ro, n) = (nid.clone(), rename_open.clone(), notepads.clone());
            spawn_local(async move {
                let _ = ApiService::rename_notepad(&nid, &new_name).await;
                ro.set(false);
                if let Ok(res) = ApiService::get_notepads().await { n.set(res.notepads_list); }
            });
        })
    };

    let on_delete_confirm = {
        let nid = (*active_notepad_id).clone();
        let delete_open = delete_open.clone();
        let active_id = active_notepad_id.clone();
        let notepads = notepads.clone();
        Callback::from(move |_| {
            let (nid, do_open, aid, n) = (nid.clone(), delete_open.clone(), active_id.clone(), notepads.clone());
            spawn_local(async move {
                let _ = ApiService::delete_notepad(&nid).await;
                do_open.set(false);
                aid.set("default".to_string());
                if let Ok(res) = ApiService::get_notepads().await { n.set(res.notepads_list); }
            });
        })
    };

    let toggle_theme = {
        let theme = theme.clone();
        Callback::from(move |_| {
            let next = match theme.as_str() {
                "light" => "dark",
                "dark" => "nord",
                "nord" => "dracula",
                "dracula" => "sepia",
                _ => "light",
            };
            StorageService::set_theme(next);
            let _ = window().and_then(|w| w.document()).and_then(|d| d.document_element()).map(|r| r.set_attribute("data-theme", next));
            theme.set(next.to_string());
        })
    };

    let on_logout = {
        let auth = authenticated.clone();
        Callback::from(move |_| {
            let auth = auth.clone();
            spawn_local(async move {
                if ApiService::logout().await.is_ok() { auth.set(false); }
            });
        })
    };

    let current_theme = StorageService::get_theme();
    let theme_stylesheet_url = if current_theme == "dark" {
        "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css"
    } else {
        "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css"
    };

    let active_name = notepads.iter().find(|n| n.id == *active_notepad_id).map(|n| n.name.clone()).unwrap_or_else(|| "default".to_string());

    let (nid_val, notes_val, ver_val, prev_val) = ((*active_notepad_id).clone(), (*notepads).clone(), (*app_version).clone(), (*preview_mode).clone());
    let on_rename_click = { let r = rename_open.clone(); Callback::from(move |_| r.set(true)) };
    let on_delete_click = { let d = delete_open.clone(); Callback::from(move |_| d.set(true)) };
    let on_preview_toggle = {
        let p = preview_mode.clone();
        Callback::from(move |_| p.set(match p.as_str() { "off" => "split", "split" => "preview-only", _ => "off" }.to_string()))
    };
    let on_search_open = { let s = search_open.clone(); Callback::from(move |_| s.set(true)) };
    let on_settings_open = { let s = settings_open.clone(); Callback::from(move |_| s.set(true)) };
    let on_shortcuts_open = { let s = shortcuts_open.clone(); Callback::from(move |_| s.set(true)) };

    html! {
        <div class="container">
            <link rel="stylesheet" href={theme_stylesheet_url} />
            <Header 
                active_notepad_id={nid_val}
                notepads={notes_val}
                on_notepad_select={on_notepad_select}
                on_new_notepad={on_new_notepad}
                on_rename={on_rename_click}
                on_delete={on_delete_click}
                preview_mode={prev_val}
                on_preview_toggle={on_preview_toggle}
                app_version={ver_val}
                on_search_open={on_search_open}
                on_settings_open={on_settings_open}
                on_shortcuts_open={on_shortcuts_open}
                toggle_theme={toggle_theme}
                on_logout={on_logout}
                current_theme={(*theme).clone()}
            />
            <main>
                <Editor 
                    notepad_id={(*active_notepad_id).clone()}
                    preview_mode={(*preview_mode).clone()}
                    save_interval={settings.save_status_message_interval}
                    disable_print_expand={settings.disable_print_expand}
                />
            </main>
            <SearchModal 
                is_open={*search_open}
                on_close={let s = search_open.clone(); Callback::from(move |_| s.set(false))}
                on_select={let active_id = active_notepad_id.clone(); Callback::from(move |id| active_id.set(id))}
            />
            <SettingsModal 
                is_open={*settings_open}
                on_close={let s = settings_open.clone(); Callback::from(move |_| s.set(false))}
                on_save={let s = settings.clone(); Callback::from(move |new_s| s.set(new_s))}
            />
            <RenameModal 
                is_open={*rename_open}
                initial_value={active_name.clone()}
                on_close={let r = rename_open.clone(); Callback::from(move |_| r.set(false))}
                on_confirm={on_rename_confirm}
            />
            <DeleteModal 
                is_open={*delete_open}
                on_close={let d = delete_open.clone(); Callback::from(move |_| d.set(false))}
                on_confirm={on_delete_confirm}
            />
            <ShortcutsModal 
                is_open={*shortcuts_open}
                on_close={let s = shortcuts_open.clone(); Callback::from(move |_| s.set(false))}
            />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
