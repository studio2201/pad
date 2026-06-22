use yew::prelude::*;
use gloo_storage::{LocalStorage, Storage};

#[derive(Clone, PartialEq)]
pub struct LocaleContext {
    pub current: String,
    pub on_change: Callback<String>,
}

impl LocaleContext {
    pub fn t(&self, key: &str) -> String {
        translate(&self.current, key)
    }
}

pub fn detect_browser_locale() -> String {
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        if let Some(lang) = navigator.language() {
            let l = lang.to_lowercase();
            if l.starts_with("zh") { return "zh".to_string(); }
            if l.starts_with("es") { return "es".to_string(); }
            if l.starts_with("de") { return "de".to_string(); }
            if l.starts_with("ja") { return "ja".to_string(); }
            if l.starts_with("fr") { return "fr".to_string(); }
            if l.starts_with("pt") { return "pt".to_string(); }
            if l.starts_with("ru") { return "ru".to_string(); }
        }
    }
    "en".to_string()
}

pub fn get_saved_locale() -> String {
    LocalStorage::get("rustpad_locale").unwrap_or_else(|_| detect_browser_locale())
}

pub fn set_saved_locale(locale: &str) {
    let _ = LocalStorage::set("rustpad_locale", locale);
}

pub fn translate(lang: &str, key: &str) -> String {
    let l = if lang.starts_with("zh") { "zh" }
            else if lang.starts_with("es") { "es" }
            else if lang.starts_with("de") { "de" }
            else if lang.starts_with("ja") { "ja" }
            else if lang.starts_with("fr") { "fr" }
            else if lang.starts_with("pt") { "pt" }
            else if lang.starts_with("ru") { "ru" }
            else { "en" };

    if let Some(val) = crate::i18n_en_es::translate_en_es(l, key) { return val.to_string(); }
    if let Some(val) = crate::i18n_de_fr::translate_de_fr(l, key) { return val.to_string(); }
    if let Some(val) = crate::i18n_ja_zh::translate_ja_zh(l, key) { return val.to_string(); }
    if let Some(val) = crate::i18n_pt_ru::translate_pt_ru(l, key) { return val.to_string(); }

    key.to_string()
}
