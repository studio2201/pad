use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub site_title: String,
    pub app_version: String,
    pub toggle_theme: Callback<MouseEvent>,
    pub on_logout: Callback<MouseEvent>,
    pub current_theme: String,
    pub is_authenticated: bool,
    pub is_pin_required: bool,
    pub disable_print: bool,
    pub enable_translation: bool,
    pub enable_themes: bool,
    pub enable_print: bool,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let locale = use_context::<crate::i18n::LocaleContext>().unwrap();

    let current_theme = &props.current_theme;
    let theme_toggle_icon = match current_theme.as_str() {
        "brinstar" => html! {
            <svg id="leaf-icon" class="leaf" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 3.5 1 9.8a7 7 0 0 1-9 8.2Z" /><path d="M19 2 9.8 11.5" /></svg>
        },
        "norfair" => html! {
            <svg id="flame-icon" class="flame" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z" /></svg>
        },
        "wrecked_ship" => html! {
            <svg id="ghost-icon" class="ghost" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 10h.01"/><path d="M15 10h.01"/><path d="M12 2a8 8 0 0 0-8 8v12l3-3 2.5 2.5L12 19l2.5 2.5L17 19l3 3V10a8 8 0 0 0-8-8z"/></svg>
        },
        "maridia" => html! {
            <svg id="waves-icon" class="waves" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 6c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" /><path d="M2 12c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" /><path d="M2 18c.6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1 .6 0 1.2-.4 1.8-1 1.2-1.2 2.4-1.2 3.6 0 .6.6 1.2 1 1.8 1" /></svg>
        },
        "tourian" => html! {
            <svg id="target-icon" class="target" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10" /><circle cx="12" cy="12" r="6" /><circle cx="12" cy="12" r="2" /></svg>
        },
        _ => html! {
            <svg id="cloud-rain-icon" class="cloud-rain" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 17.58A5 5 0 0 0 18 8h-1.26A8 8 0 1 0 4 16.25" /><path d="M8 20v2" /><path d="M12 20v2" /><path d="M16 20v2" /></svg>
        },
    };

    let on_lang_change = {
        let locale = locale.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            locale.on_change.emit(select.value());
        })
    };

    let langs = [
        ("en", "English"),
        ("zh", "简体中文"),
        ("es", "Español"),
        ("de", "Deutsch"),
        ("ja", "日本語"),
        ("fr", "Français"),
        ("pt", "Português"),
        ("ru", "Русский"),
    ];

    let theme_toggle_tooltip = match locale.current.as_str() {
        "zh" => "切换主题",
        "es" => "Cambiar tema",
        "de" => "Design umschalten",
        "ja" => "テーマ切り替え",
        "fr" => "Changer de thème",
        "pt" => "Alternar tema",
        "ru" => "Переключить тему",
        _ => "Toggle theme",
    };

    let print_tooltip = match locale.current.as_str() {
        "zh" => "打印",
        "es" => "Imprimir",
        "de" => "Drucken",
        "ja" => "印刷",
        "fr" => "Imprimer",
        "pt" => "Imprimir",
        "ru" => "Печать",
        _ => "Print",
    };

    let on_print = Callback::from(|_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    });

    html! {
        <header>
            <div id="header-title">
                <h1>{&props.site_title}</h1>
            </div>
            <div class="header-right">
                {if props.enable_translation {
                    html! {
                        <div class="language-select-container">
                            <select
                                class="language-select"
                                id="language-select"
                                value={locale.current.clone()}
                                onchange={on_lang_change}
                                aria-label="Select language"
                            >
                                {for langs.iter().map(|&(code, label)| {
                                    html! {
                                        <option value={code} selected={locale.current == code}>
                                            {label}
                                        </option>
                                    }
                                })}
                            </select>
                        </div>
                    }
                } else {
                    html! {}
                }}
                {if props.enable_themes {
                    html! {
                        <button id="theme-toggle" class="icon-button" onclick={props.toggle_theme.clone()} aria-label="Toggle theme" title={theme_toggle_tooltip}>
                            {theme_toggle_icon}
                        </button>
                    }
                } else {
                    html! {}
                }}
                {if props.enable_print {
                    html! {
                        <button
                            id="print-button"
                            class="icon-button"
                            onclick={on_print}
                            disabled={props.disable_print}
                            title={print_tooltip}
                        >
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <polyline points="6 9 6 2 18 2 18 9" />
                                <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
                                <rect x="6" y="14" width="12" height="8" />
                            </svg>
                        </button>
                    }
                } else {
                    html! {}
                }}
                {if props.is_pin_required {
                    html! {
                        <button
                            id="logout-button"
                            class="icon-button"
                            onclick={props.on_logout.clone()}
                            disabled={!props.is_authenticated || !props.is_pin_required}
                            data-tooltip={if !props.is_authenticated || !props.is_pin_required { "".to_string() } else { locale.t("logout") }}
                        >
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><polyline points="16 17 21 12 16 7" /><line x1="21" y1="12" x2="9" y2="12" /></svg>
                        </button>
                    }
                } else {
                    html! {}
                }}
            </div>
        </header>
    }
}
