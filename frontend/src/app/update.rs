use crate::app::{App, Msg};
use crate::api::{ApiService, StorageService};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use shared_assets::theme::Theme;


impl App {
    pub fn create_app(ctx: &Context<Self>) -> Self {
        let theme = StorageService::get_theme();
        let locale_state = crate::i18n::get_saved_locale();

        if let Some(win) = web_sys::window()
            && let Some(doc) = win.document()
            && let Some(el) = doc.document_element()
        {
            let _ = el.set_attribute("data-theme", &theme);
            let _ = el.set_attribute("class", &theme);
        }

        let link = ctx.link().clone();
        spawn_local(async move {
            if let Ok(config) = ApiService::get_config().await {
                link.send_message(Msg::LoadConfig(config));
            }
        });

        let link = ctx.link().clone();
        spawn_local(async move {
            if let Ok(res) = ApiService::check_pin_required().await {
                link.send_message(Msg::LoadPinRequired(res.required));
            }
        });

        Self {
            authenticated: false,
            app_version: "2.0.0".to_string(),
            site_title: "Pad".to_string(),
            theme,
            locale_state,
            active_notification: None,
            is_pin_required: true,
            enable_translation: false,
            enable_themes: true,
            enable_print: false,
            show_version: true,
            show_github: true,
            is_content_empty: true,
        }
    }

    pub fn update_app(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::LoadConfig(config) => {
                self.app_version = config.version;
                self.site_title = config.site_title.clone();
                self.enable_translation = config.enable_translation;
                self.enable_themes = config.enable_themes;
                self.enable_print = config.enable_print;
                self.show_version = config.show_version;
                self.show_github = config.show_github;
                if !config.enable_themes {
                    self.theme = "tourian".to_string();
                    StorageService::set_theme("tourian");
                    if let Some(win) = web_sys::window()
                        && let Some(doc) = win.document()
                        && let Some(el) = doc.document_element()
                    {
                        let _ = el.set_attribute("data-theme", "tourian");
                        let _ = el.set_attribute("class", "tourian");
                    }
                }
                if let Some(win) = web_sys::window()
                    && let Some(doc) = win.document()
                {
                    doc.set_title(&config.site_title);
                }
                true
            }
            Msg::LoadPinRequired(req) => {
                self.is_pin_required = req;
                true
            }
            Msg::SetAuthenticated(auth) => {
                self.authenticated = auth;
                if auth {
                    spawn_local(async move {
                        // Fetch default notes to make sure default notepad is initialized
                        let _ = ApiService::get_notes("default").await;
                    });
                }
                true
            }
            Msg::SwitchLanguage(lang) => {
                crate::i18n::set_saved_locale(&lang);
                self.locale_state = lang;
                true
            }
            Msg::ToggleTheme => {
                let current = Theme::from_name(&self.theme).unwrap_or_default();
                let next = match current {
                    Theme::Brinstar => Theme::Norfair,
                    Theme::Norfair => Theme::WreckedShip,
                    Theme::WreckedShip => Theme::Maridia,
                    Theme::Maridia => Theme::Tourian,
                    Theme::Tourian => Theme::Crateria,
                    Theme::Crateria => Theme::Brinstar,
                };
                StorageService::set_theme(next.name());
                if let Some(html) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.document_element())
                {
                    let _ = html.set_attribute("data-theme", next.name());
                    let _ = html.set_attribute("class", next.name());
                }
                self.theme = next.name().to_string();
                true
            }

            Msg::Logout => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if ApiService::logout().await.is_ok() {
                        link.send_message(Msg::SetAuthenticated(false));
                    }
                });
                false
            }
            Msg::SetStatus(status) => {
                self.active_notification = status;
                true
            }
            Msg::SetContentEmpty(is_empty) => {
                self.is_content_empty = is_empty;
                true
            }
        }
    }
}
