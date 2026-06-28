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
}
