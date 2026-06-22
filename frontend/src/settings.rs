use yew::prelude::*;
use crate::types::Settings;
use crate::services::StorageService;

#[derive(Properties, PartialEq)]
pub struct SettingsModalProps {
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub on_save: Callback<Settings>,
}

#[function_component(SettingsModal)]
pub fn settings_modal(props: &SettingsModalProps) -> Html {
    let settings = use_state(StorageService::get_settings);

    // Sync settings when modal opens
    {
        let settings = settings.clone();
        let is_open = props.is_open;
        use_effect_with(is_open, move |&open| {
            if open {
                settings.set(StorageService::get_settings());
            }
            || ()
        });
    }

    if !props.is_open {
        return html! {};
    }

    let on_save = {
        let settings = settings.clone();
        let on_save_cb = props.on_save.clone();
        let on_close_cb = props.on_close.clone();
        Callback::from(move |_| {
            StorageService::set_settings(&settings);
            on_save_cb.emit((*settings).clone());
            on_close_cb.emit(());
        })
    };

    let on_reset = {
        let settings = settings.clone();
        Callback::from(move |_| {
            let defaults = Settings::default();
            settings.set(defaults);
        })
    };

    let on_close = {
        let on_close_cb = props.on_close.clone();
        Callback::from(move |_| {
            on_close_cb.emit(());
        })
    };

    let on_interval_input = {
        let settings = settings.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let val = input.value().parse::<u64>().unwrap_or(0);
            let mut s = (*settings).clone();
            s.save_status_message_interval = val;
            settings.set(s);
        })
    };

    let on_remote_messages_change = {
        let settings = settings.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut s = (*settings).clone();
            s.enable_remote_connection_messages = input.checked();
            settings.set(s);
        })
    };

    let on_preview_mode_change = {
        let settings = settings.clone();
        Callback::from(move |mode: String| {
            let mut s = (*settings).clone();
            s.default_markdown_preview_mode = mode;
            settings.set(s);
        })
    };

    let on_disable_print_expand_change = {
        let settings = settings.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut s = (*settings).clone();
            s.disable_print_expand = input.checked();
            settings.set(s);
        })
    };

    html! {
        <div id="settings-modal" class="modal visible">
            <div class="modal-content">
                <h2>{"Settings"}</h2>
                <div class="settings-form">
                    <label class="settings-label">
                        {"Status Message Timing (ms) - Autosave:"}
                        <input 
                            id="autosave-status-interval-input" 
                            class="modal-input" 
                            type="number" 
                            min="0" 
                            value={settings.save_status_message_interval.to_string()} 
                            oninput={on_interval_input}
                            placeholder="Leave empty or 0 to disable messages" 
                        />
                    </label>
                    <label class="settings-label">
                        {"Enable Remote Connection Messages:"}
                        <input 
                            type="checkbox" 
                            id="settings-remote-connection-messages" 
                            checked={settings.enable_remote_connection_messages}
                            onchange={on_remote_messages_change}
                        />
                    </label>
                    <label class="settings-label">
                        {"Default preview mode:"}
                        <div style="margin-top: 0.5rem; display: flex; gap: 1rem;">
                            <label style="display: flex; align-items: center; gap: 0.5rem;">
                                <input 
                                    type="radio" 
                                    name="default-preview-mode" 
                                    value="off" 
                                    checked={settings.default_markdown_preview_mode == "off"}
                                    onclick={let m_c = on_preview_mode_change.clone(); move |_| m_c.emit("off".to_string())}
                                />
                                {"Editor"}
                            </label>
                            <label style="display: flex; align-items: center; gap: 0.5rem;">
                                <input 
                                    type="radio" 
                                    name="default-preview-mode" 
                                    value="split" 
                                    checked={settings.default_markdown_preview_mode == "split"}
                                    onclick={let m_c = on_preview_mode_change.clone(); move |_| m_c.emit("split".to_string())}
                                />
                                {"Split"}
                            </label>
                            <label style="display: flex; align-items: center; gap: 0.5rem;">
                                <input 
                                    type="radio" 
                                    name="default-preview-mode" 
                                    value="preview-only" 
                                    checked={settings.default_markdown_preview_mode == "preview-only"}
                                    onclick={let m_c = on_preview_mode_change.clone(); move |_| m_c.emit("preview-only".to_string())}
                                />
                                {"Full"}
                            </label>
                        </div>
                    </label>
                    <label class="settings-label">
                        {"Disable auto-expand markdown in print:"}
                        <input 
                            type="checkbox" 
                            id="settings-disable-print-expand" 
                            checked={settings.disable_print_expand}
                            onchange={on_disable_print_expand_change}
                        />
                    </label>
                </div>
                <div class="modal-buttons">
                    <button id="settings-cancel" onclick={on_close}>{"Cancel"}</button>
                    <button id="settings-reset" class="danger" onclick={on_reset}>{"Reset"}</button>
                    <button id="settings-save" onclick={on_save}>{"Save"}</button>
                </div>
            </div>
        </div>
    }
}
