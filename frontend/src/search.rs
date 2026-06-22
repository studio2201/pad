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
                            let q_val = (*query).clone();
                            
                            let on_click = Callback::from(move |_| {
                                on_select_cb.emit(item_id.clone());
                                query_state.set("".to_string());
                                results_state.set(vec![]);
                                on_close_cb.emit(());
                            });
                            
                            html! {
                                <li onclick={on_click} style="cursor: pointer;">
                                    <div>{highlight_query(&item.name, &q_val)}</div>
                                    <div style="font-size: 0.85em; opacity: 0.7; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                        {format!("[{}]", item.r#match)}
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

fn highlight_query(text: &str, query: &str) -> Html {
    if query.is_empty() {
        return html! { <>{text}</> };
    }

    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.to_lowercase().chars().collect();
    let text_lower_chars: Vec<char> = text.to_lowercase().chars().collect();

    let mut result = Vec::new();
    let mut i = 0;
    let text_len = text_chars.len();
    let query_len = query_chars.len();

    while i < text_len {
        let mut is_match = false;
        if i + query_len <= text_len {
            is_match = true;
            for j in 0..query_len {
                if text_lower_chars[i + j] != query_chars[j] {
                    is_match = false;
                    break;
                }
            }
        }

        if is_match {
            let matched_str: String = text_chars[i..i + query_len].iter().collect();
            result.push(html! { <mark>{matched_str}</mark> });
            i += query_len;
        } else {
            result.push(html! { {text_chars[i].to_string()} });
            i += 1;
        }
    }

    html! { <>{for result}</> }
}
