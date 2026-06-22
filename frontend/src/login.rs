use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::services::{ApiService, StorageService};

#[derive(Properties, PartialEq)]
pub struct LoginProps {
    pub on_login_success: Callback<()>,
}

#[function_component(Login)]
pub fn login(props: &LoginProps) -> Html {
    let pin_input = use_state(|| "".to_string());
    let error_msg = use_state(|| "".to_string());
    let is_locked = use_state(|| false);
    let pin_length = use_state(|| 4);
    let theme = use_state(StorageService::get_theme);
    let input_ref = use_node_ref();

    {
        let input_ref = input_ref.clone();
        use_effect_with((*is_locked).clone(), move |locked| {
            if !*locked {
                if let Some(input) = input_ref.cast::<web_sys::HtmlInputElement>() {
                    let _ = input.focus();
                }
            }
            || ()
        });
    }

    {
        let on_success = props.on_login_success.clone();
        let is_locked = is_locked.clone();
        let pin_length = pin_length.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ApiService::check_pin_required().await {
                    if !res.required {
                        on_success.emit(());
                    } else {
                        is_locked.set(res.locked);
                        pin_length.set(res.length);
                    }
                }
            });
            || ()
        });
    }

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
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(root) = doc.document_element() {
                    let _ = root.set_attribute("data-theme", next);
                }
            }
            theme.set(next.to_string());
        })
    };

    let on_input = {
        let pin_input = pin_input.clone();
        let pin_len = *pin_length;
        let on_success = props.on_login_success.clone();
        let error_msg = error_msg.clone();
        let is_locked = is_locked.clone();
        
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let val = input.value();
            let filtered: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
            input.set_value(&filtered);

            if filtered.len() <= pin_len {
                pin_input.set(filtered.clone());
                error_msg.set("".to_string());
                
                if filtered.len() == pin_len {
                    let on_success = on_success.clone();
                    let error_msg = error_msg.clone();
                    let is_locked = is_locked.clone();
                    let val_clone = filtered.clone();
                    
                    spawn_local(async move {
                        if let Ok(res) = ApiService::verify_pin(&val_clone).await {
                            if res.success {
                                on_success.emit(());
                            } else {
                                if let Some(err) = res.error {
                                    if err.contains("Too many attempts") {
                                        is_locked.set(true);
                                    }
                                    error_msg.set(err);
                                } else {
                                    error_msg.set("Invalid PIN".to_string());
                                }
                            }
                        }
                    });
                }
            }
        })
    };

    let on_submit = {
        let pin_input = pin_input.clone();
        let pin_len = *pin_length;
        let on_success = props.on_login_success.clone();
        let error_msg = error_msg.clone();
        let is_locked = is_locked.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let val = (*pin_input).clone();
            if val.len() == pin_len {
                let on_success = on_success.clone();
                let error_msg = error_msg.clone();
                let is_locked = is_locked.clone();
                spawn_local(async move {
                    if let Ok(res) = ApiService::verify_pin(&val).await {
                        if res.success {
                            on_success.emit(());
                        } else {
                            if let Some(err) = res.error {
                                if err.contains("Too many attempts") {
                                    is_locked.set(true);
                                }
                                error_msg.set(err);
                            } else {
                                error_msg.set("Invalid PIN".to_string());
                            }
                        }
                    }
                });
            }
        })
    };

    let theme_toggle_icon = match theme.as_str() {
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

    html! {
        <div class="container login-container">
            <button id="theme-toggle" class="theme-toggle" onclick={toggle_theme} aria-label="Toggle dark mode">
                {theme_toggle_icon}
            </button>
            <div id="login-content">
                <div class="pin-header">
                    <h1 id="site-title">{"RustPad"}</h1>
                    <h2 id="pin-description">
                        {if *is_locked { "Locked Out" } else { "Enter PIN" }}
                    </h2>
                </div>
                <form id="pin-form" onsubmit={on_submit}>
                    <div class="pin-wrapper">
                        <input 
                            ref={input_ref.clone()}
                            type="password" 
                            class="pin-input-field" 
                            value={(*pin_input).clone()}
                            oninput={on_input}
                            disabled={*is_locked}
                            placeholder={"• ".repeat(*pin_length).trim().to_string()}
                            maxlength={pin_length.to_string()}
                            autofocus=true
                        />
                    </div>
                </form>
                <p id="pin-error" class="error-message">
                    {(*error_msg).clone()}
                </p>
            </div>
        </div>
    }
}
