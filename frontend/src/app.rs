pub mod update;
pub mod view;

use crate::api::ConfigResponse;
use yew::prelude::*;

pub enum Msg {
    LoadConfig(ConfigResponse),
    LoadPinRequired(bool),
    SetAuthenticated(bool),
    SwitchLanguage(String),
    ToggleTheme,
    Logout,
    SetStatus(Option<(String, String)>),
    SetContentEmpty(bool),
    OnlineStatusChanged(bool),
    Print,
}

pub struct App {
    pub authenticated: bool,
    pub app_version: String,
    pub site_title: String,
    pub theme: String,
    pub locale_state: String,
    pub active_notification: Option<(String, String)>,
    pub is_pin_required: bool,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
    pub show_version: bool,
    pub show_github: bool,
    pub is_content_empty: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self::create_app(ctx)
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        self.update_app(ctx, msg)
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        self.view_app(ctx)
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            use wasm_bindgen::JsCast;
            if let Some(window) = web_sys::window() {
                let link_online = ctx.link().clone();
                let on_online = wasm_bindgen::prelude::Closure::<dyn FnMut(_)>::new(
                    move |_: web_sys::Event| {
                        link_online.send_message(Msg::OnlineStatusChanged(true));
                    },
                );
                let _ = window
                    .add_event_listener_with_callback("online", on_online.as_ref().unchecked_ref());
                on_online.forget();

                let link_offline = ctx.link().clone();
                let on_offline = wasm_bindgen::prelude::Closure::<dyn FnMut(_)>::new(
                    move |_: web_sys::Event| {
                        link_offline.send_message(Msg::OnlineStatusChanged(false));
                    },
                );
                let _ = window.add_event_listener_with_callback(
                    "offline",
                    on_offline.as_ref().unchecked_ref(),
                );
                on_offline.forget();
            }
        }
    }
}
