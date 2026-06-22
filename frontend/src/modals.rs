use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct RenameModalProps {
    pub is_open: bool,
    pub initial_value: String,
    pub on_close: Callback<()>,
    pub on_confirm: Callback<String>,
}

#[function_component(RenameModal)]
pub fn rename_modal(props: &RenameModalProps) -> Html {
    let rename_value = use_state(|| props.initial_value.clone());

    {
        let rename_value = rename_value.clone();
        let initial_value = props.initial_value.clone();
        use_effect_with(initial_value, move |val| {
            rename_value.set(val.clone());
            || ()
        });
    }

    if !props.is_open {
        return html! {};
    }

    let on_input = {
        let rename_value = rename_value.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            rename_value.set(input.value());
        })
    };

    let on_confirm_click = {
        let on_confirm = props.on_confirm.clone();
        let rename_value = rename_value.clone();
        Callback::from(move |_| {
            on_confirm.emit((*rename_value).clone());
        })
    };

    let on_cancel_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    html! {
        <div id="rename-modal" class="modal visible">
            <div class="modal-content">
                <h2>{"Rename Notepad"}</h2>
                <input 
                    type="text" 
                    class="modal-input" 
                    value={(*rename_value).clone()}
                    oninput={on_input}
                />
                <div class="modal-buttons">
                    <button onclick={on_cancel_click}>{"Cancel"}</button>
                    <button onclick={on_confirm_click}>{"Rename"}</button>
                </div>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct DeleteModalProps {
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub on_confirm: Callback<()>,
}

#[function_component(DeleteModal)]
pub fn delete_modal(props: &DeleteModalProps) -> Html {
    if !props.is_open {
        return html! {};
    }

    let on_cancel_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    let on_confirm_click = {
        let on_confirm = props.on_confirm.clone();
        Callback::from(move |_| {
            on_confirm.emit(());
        })
    };

    html! {
        <div id="delete-modal" class="modal visible">
            <div class="modal-content">
                <h2>{"Delete Notepad"}</h2>
                <p class="modal-message">{"Are you sure you want to delete this notepad? This action cannot be undone."}</p>
                <div class="modal-buttons">
                    <button onclick={on_cancel_click}>{"Cancel"}</button>
                    <button class="danger" onclick={on_confirm_click}>{"Delete"}</button>
                </div>
            </div>
        </div>
    }
}
