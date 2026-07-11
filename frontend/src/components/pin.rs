use crate::api::ApiService;
use shared_frontend::i18n::Language;
use shared_frontend::i18n::strings::{StringKey, lookup};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LoginProps {
    pub on_login_success: Callback<()>,
    #[prop_or_default]
    pub on_status_change: Callback<Option<(String, String)>>,
}

#[function_component(Login)]
pub fn login(props: &LoginProps) -> Html {
    let pin_input = use_state(|| "".to_string());
    let error_msg = use_state(|| "".to_string());
    let is_locked = use_state(|| false);
    let pin_length = use_state(|| 4);
    let input_ref = use_node_ref();
    let locale = use_context::<crate::i18n::LocaleContext>().unwrap();

    {
        let input_ref = input_ref.clone();
        use_effect_with(*is_locked, move |locked| {
            if !*locked && let Some(input) = input_ref.cast::<web_sys::HtmlInputElement>() {
                let _ = input.focus();
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

    let on_input = {
        let pin_input = pin_input.clone();
        let pin_len = *pin_length;
        let on_success = props.on_login_success.clone();
        let on_status = props.on_status_change.clone();
        let error_msg = error_msg.clone();
        let is_locked = is_locked.clone();
        let locale = locale.clone();

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
                    let on_status = on_status.clone();
                    let is_locked = is_locked.clone();
                    let error_msg = error_msg.clone();
                    let loc_code = locale.current.clone();

                    spawn_local(async move {
                        if let Ok(res) = ApiService::verify_pin(&filtered).await {
                            if res.success {
                                on_success.emit(());
                            } else {
                                let status_msg = lookup(
                                    StringKey::StatusPinFailure,
                                    Language::from_code(&loc_code),
                                )
                                .to_string();
                                on_status.emit(Some((status_msg, "error".to_string())));
                                let on_status_clear = on_status.clone();
                                gloo_timers::callback::Timeout::new(3000, move || {
                                    on_status_clear.emit(None);
                                })
                                .forget();

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
        let on_status = props.on_status_change.clone();
        let loc_val = locale.current.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let val = (*pin_input).clone();
            if val.len() == pin_len {
                let on_success = on_success.clone();
                let error_msg = error_msg.clone();
                let is_locked = is_locked.clone();
                let on_status = on_status.clone();
                let loc_code = loc_val.clone();
                spawn_local(async move {
                    if let Ok(res) = ApiService::verify_pin(&val).await {
                        if res.success {
                            on_success.emit(());
                        } else {
                            let status_msg =
                                lookup(StringKey::StatusPinFailure, Language::from_code(&loc_code))
                                    .to_string();
                            on_status.emit(Some((status_msg, "error".to_string())));
                            let on_status_clear = on_status.clone();
                            gloo_timers::callback::Timeout::new(3000, move || {
                                on_status_clear.emit(None);
                            })
                            .forget();

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

    html! {
        <div class="login-container">
            <div class="login-box">
                <div class="pin-header">
                    <h2 id="pin-description">
                        {if *is_locked { locale.t("login_locked") } else { locale.t("login_prompt") }}
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
                <div class="pin-status">
                    if !(*error_msg).is_empty() {
                        <p id="pin-error" class="pin-error" style="display: block;">
                            {(*error_msg).clone()}
                        </p>
                    }
                </div>
            </div>
        </div>
    }
}
