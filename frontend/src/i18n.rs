use yew::prelude::*;

pub use shared_frontend::locale::{get_saved_locale, set_saved_locale};

mod de;
mod en;
mod es;
mod fr;
mod ja;
mod pt;
mod ru;
mod zh;

#[derive(Clone, PartialEq)]
pub struct LocaleContext {
    pub current: String,
    pub on_change: Callback<String>,
}

impl Default for LocaleContext {
    fn default() -> Self {
        Self {
            current: "en".to_string(),
            on_change: Callback::noop(),
        }
    }
}

impl LocaleContext {
    pub fn t(&self, key: &str) -> String {
        translate(&self.current, key)
    }
}

pub fn translate(lang: &str, key: &str) -> String {
    let l = if lang.starts_with("zh") {
        "zh"
    } else if lang.starts_with("es") {
        "es"
    } else if lang.starts_with("de") {
        "de"
    } else if lang.starts_with("ja") {
        "ja"
    } else if lang.starts_with("fr") {
        "fr"
    } else if lang.starts_with("pt") {
        "pt"
    } else if lang.starts_with("ru") {
        "ru"
    } else {
        "en"
    };

    let val = match l {
        "zh" => zh::translate(key),
        "es" => es::translate(key),
        "de" => de::translate(key),
        "ja" => ja::translate(key),
        "fr" => fr::translate(key),
        "pt" => pt::translate(key),
        "ru" => ru::translate(key),
        _ => en::translate(key),
    };

    val.unwrap_or(key).to_string()
}
