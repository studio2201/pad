use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::services::ApiService;
use crate::types::SearchItem;

#[derive(Properties, PartialEq)]
pub struct SearchModalProps {
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub on_select: Callback<String>,
}

#[function_component(SearchModal)]
pub fn search_modal(props: &SearchModalProps) -> Html {
    let query = use_state(|| "".to_string());
    let results = use_state(|| Vec::<SearchItem>::new());
    let search_input_ref = use_node_ref();

    {
        let is_open = props.is_open;
        let input_ref = search_input_ref.clone();
        use_effect_with(is_open, move |&open| {
            if open {
                if let Some(input) = input_ref.cast::<web_sys::HtmlInputElement>() {
                    let _ = input.focus();
                }
            }
            || ()
        });
    }

    {
        let query_val = (*query).clone();
        let results = results.clone();
        let is_open = props.is_open;
        use_effect_with((query_val, is_open), move |(q, open)| {
            if *open {
                let q = q.clone();
                let results = results.clone();
                spawn_local(async move {
                    if q.trim().is_empty() {
                        results.set(vec![]);
                        return;
                    }
                    if let Ok(res) = ApiService::search(&q).await {
                        results.set(res.results);
                    }
                });
            }
            || ()
        });
    }

    if !props.is_open {
        return html! {};
    }

    let on_close = {
        let on_close_cb = props.on_close.clone();
        let query = query.clone();
        let results = results.clone();
        Callback::from(move |_| {
            query.set("".to_string());
            results.set(vec![]);
            on_close_cb.emit(());
        })
    };

    let on_input = {
        let query = query.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            query.set(input.value());
        })
    };

    let on_keydown = {
        let on_close_cb = props.on_close.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Escape" {
                on_close_cb.emit(());
            }
        })
    };

    html! {
        <div id="search-modal" class="modal visible" onclick={on_close.clone()}>
            <div class="search-modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                <input 
                    type="text" 
                    id="search-box" 
                    ref={search_input_ref}
                    placeholder="Search notes..." 
                    value={(*query).clone()}
                    oninput={on_input}
                    onkeydown={on_keydown}
                    autofocus=true 
                />
                <ul id="search-results">
                    {
                        for results.iter().map(|item| {
                            let item_id = item.id.clone();
                            let on_select_cb = props.on_select.clone();
                            let on_close_cb = props.on_close.clone();
                            let query_state = query.clone();
                            let results_state = results.clone();
                            
                            let on_click = Callback::from(move |_| {
                                on_select_cb.emit(item_id.clone());
                                query_state.set("".to_string());
                                results_state.set(vec![]);
                                on_close_cb.emit(());
                            });
                            
                            html! {
                                <li onclick={on_click} style="cursor: pointer;">
                                    <strong>{&item.name}</strong>
                                    <div style="font-size: 0.85em; opacity: 0.7; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                        {&item.content}
                                    </div>
                                </li>
                            }
                        })
                    }
                </ul>
            </div>
        </div>
    }
}
