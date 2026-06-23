use crate::editor::Editor;
use crate::header::Header;
use crate::login::Login;
use crate::services::{ApiService, StorageService};
use wasm_bindgen_futures::spawn_local;
use web_sys::window;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let authenticated = use_state(|| false);
    let app_version = use_state(|| "1.0.6".to_string());
    let site_title = use_state(|| "RustPad".to_string());
    let theme = use_state(StorageService::get_theme);
    let locale_state = use_state(crate::i18n::get_saved_locale);
    let active_notification = use_state(|| None::<(String, String)>);
    let is_pin_required = use_state(|| true);

    {
        let version = app_version.clone();
        let site_title = site_title.clone();
        let pin_req = is_pin_required.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(config) = ApiService::get_config().await {
                    version.set(config.version);
                    site_title.set(config.site_title.clone());
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            doc.set_title(&config.site_title);
                        }
                    }
                }
            });
            spawn_local(async move {
                if let Ok(res) = ApiService::check_pin_required().await {
                    pin_req.set(res.required);
                }
            });
            || ()
        });
    }

    {
        let authenticated = authenticated.clone();

        use_effect_with(*authenticated, move |&auth| {
            if auth {
                spawn_local(async move {
                    // Fetch default notes to make sure default notepad is initialized
                    let _ = ApiService::get_notes("default").await;
                });
            }
            || ()
        });
    }

    let locale_on_change = {
        let ls = locale_state.clone();
        Callback::from(move |new_lang: String| {
            crate::i18n::set_saved_locale(&new_lang);
            ls.set(new_lang);
        })
    };
    let locale_context = crate::i18n::LocaleContext {
        current: (*locale_state).clone(),
        on_change: locale_on_change,
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
            if let Some(html) = window()
                .and_then(|w| w.document())
                .and_then(|d| d.document_element())
            {
                let _ = html.set_attribute("data-theme", next);
                let _ = html.set_attribute("class", next);
            }
            theme.set(next.to_string());
        })
    };

    let on_logout = {
        let auth = authenticated.clone();
        Callback::from(move |_| {
            let auth = auth.clone();
            spawn_local(async move {
                if ApiService::logout().await.is_ok() {
                    auth.set(false);
                }
            });
        })
    };

    let ver_val = (*app_version).clone();

    html! {
        <ContextProvider<crate::i18n::LocaleContext> context={locale_context}>
            <Header
                site_title={(*site_title).clone()}
                app_version={ver_val}
                toggle_theme={toggle_theme}
                on_logout={on_logout}
                current_theme={(*theme).clone()}
                is_authenticated={*authenticated}
                is_pin_required={*is_pin_required}
            />
            <div class="container">
                {if !*authenticated {
                    html! { <Login on_login_success={
                        let auth = authenticated.clone();
                        Callback::from(move |_| {
                            auth.set(true);
                            if let Some(win) = web_sys::window() {
                                let loc = win.location();
                                let search = loc.search().unwrap_or_default();
                                let mut redirect_url = "/".to_string();
                                if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                                    if let Some(r) = params.get("redirect") {
                                        if !r.is_empty() && r.starts_with('/') && !r.starts_with("//") {
                                            redirect_url = r;
                                        }
                                    }
                                }
                                if let Ok(history) = win.history() {
                                    let _ = history.replace_state_with_url(
                                        &wasm_bindgen::JsValue::NULL,
                                        "",
                                        Some(&redirect_url),
                                    );
                                }
                            }
                        })
                    } /> }
                } else {
                    html! {
                        <main>
                            <Editor
                                notepad_id={"default".to_string()}
                                save_interval={3000}
                                disable_print_expand={false}
                                on_status={let active_notif = active_notification.clone(); Callback::from(move |status| active_notif.set(status))}
                            />
                        </main>
                    }
                }}
            </div>
            <footer class="layout-footer">
                {
                    if let Some((msg, cls)) = &*active_notification {
                        html! { <div class={format!("footer-status-text {}", cls)}>{ msg }</div> }
                    } else {
                        html! {}
                    }
                }
            </footer>
        </ContextProvider<crate::i18n::LocaleContext>>
    }
}
