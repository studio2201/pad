//! Backward-compatible `Login` wrapper.
//!
//! Apps written against the old `Login { on_login_success, on_status_change }`
//! API keep working unchanged; the wrapper handles the API polling, the
//! auto-submit, and the verify-pin call internally, then renders the
//! shared [`shared_frontend::components::Login`] with the right props.
//!
//! Apps that want to do their own state management can switch to the shared
//! `Login` directly; this wrapper exists only to keep the call sites
//! working while we roll out the new component.

use crate::api::ApiService;
use shared_frontend::components::login::Login as SharedLogin;
use shared_frontend::i18n::Language;
use shared_frontend::i18n::strings::{StringKey, lookup};
use shared_frontend::locale::get_saved_locale;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// Props for the legacy [`Login`] component. Apps written against the
/// old API can keep using these.
#[derive(Properties, PartialEq)]
pub struct LoginProps {
    /// Fires once the backend accepts the entered PIN. Passes `true` if a
    /// PIN was verified, `false` if the login form was bypassed because
    /// no PIN is configured.
    pub on_login_success: Callback<bool>,
    /// Optional sink for transient status banners; defaults to a no-op
    /// callback so unconfigured parents don't have to pass anything.
    #[prop_or_default]
    pub on_status_change: Callback<Option<(String, String)>>,
}

/// Legacy login component. Internally renders the shared
/// [`SharedLogin`] after polling `/api/pin-required` for the expected
/// length and lockout state.
#[function_component(Login)]
pub fn login(props: &LoginProps) -> Html {
    let pin_length = use_state(|| 4_usize);
    let locked = use_state(|| false);
    let on_success = props.on_login_success.clone();
    let on_status = props.on_status_change.clone();

    // Determine display language from the saved locale cookie (falls
    // back to `en` if no cookie is set).
    let language = Language::from_code(&get_saved_locale().unwrap_or_else(|| "en".to_string()));

    // Poll the backend for PIN-required metadata on mount.
    {
        let pin_length = pin_length.clone();
        let locked = locked.clone();
        let on_success = on_success.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ApiService::check_pin_required().await {
                    if !res.required {
                        on_success.emit(false);
                    } else {
                        if res.length > 0 {
                            pin_length.set(res.length);
                        }
                        locked.set(res.locked);
                    }
                }
            });
            || ()
        });
    }

    let on_verify = {
        let on_success = on_success.clone();
        let on_status = on_status.clone();
        let locked = locked.clone();
        let language = language;
        Callback::from(move |pin: String| {
            let on_success = on_success.clone();
            let on_status = on_status.clone();
            let locked = locked.clone();
            let language = language;
            spawn_local(async move {
                match ApiService::verify_pin(&pin).await {
                    Ok(res) if res.success => {
                        on_success.emit(true);
                    }
                    Ok(res) => {
                        let msg = lookup(
                            StringKey::StatusPinFailure,
                            language,
                        )
                        .to_string();
                        on_status.emit(Some((msg, "error".to_string())));
                        let on_status = on_status.clone();
                        gloo_timers::callback::Timeout::new(3000, move || {
                            on_status.emit(None);
                        })
                        .forget();
                        if res
                            .error
                            .as_deref()
                            .is_some_and(|e| e.contains("Too many attempts"))
                        {
                            locked.set(true);
                        }
                    }
                    Err(_) => {
                        on_status.emit(Some((
                            lookup(StringKey::StatusLoadError, language).to_string(),
                            "error".to_string(),
                        )));
                    }
                }
            });
        })
    };

    let prompt = lookup(
        StringKey::TitleViewReleaseNotes,
        language,
    );
    let locked_text = lookup(StringKey::AriaSelectLanguage, language);

    html! {
        <SharedLogin
            pin_required={true}
            pin_length={*pin_length}
            locked={*locked}
            on_verify={on_verify}
            on_login_success={on_success}
            prompt_text={prompt}
            locked_text={locked_text}
            language={Some(language)}
        />
    }
}
