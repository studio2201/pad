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
                                    <strong>{&item.name}</strong>
                                    <div style="font-size: 0.85em; opacity: 0.7; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                        {render_snippet(&item.content, &q_val)}
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

fn render_snippet(content: &str, query: &str) -> Html {
    if query.is_empty() {
        let clean_content: String = content.chars().take(100).map(|c| if c == '\n' { ' ' } else { c }).collect();
        return html! { <span>{clean_content}{if content.chars().count() > 100 { "..." } else { "" }}</span> };
    }

    let clean_content: String = content.replace('\n', " ");
    let query_lower = query.to_lowercase();
    let content_lower = clean_content.to_lowercase();

    if let Some(idx) = content_lower.find(&query_lower) {
        let char_indices: Vec<(usize, char)> = clean_content.char_indices().collect();
        let char_idx = char_indices.iter().position(|&(byte_idx, _)| byte_idx == idx).unwrap_or(0);
        
        let start_char = if char_idx > 30 { char_idx - 30 } else { 0 };
        let end_char = std::cmp::min(char_indices.len(), char_idx + query.chars().count() + 60);

        let prefix: String = char_indices[start_char..char_idx].iter().map(|&(_, c)| c).collect();
        let matched: String = char_indices[char_idx..char_idx + query.chars().count()].iter().map(|&(_, c)| c).collect();
        let suffix: String = char_indices[char_idx + query.chars().count()..end_char].iter().map(|&(_, c)| c).collect();

        html! {
            <span>
                {if start_char > 0 { "..." } else { "" }}
                {prefix}
                <mark>{matched}</mark>
                {suffix}
                {if end_char < char_indices.len() { "..." } else { "" }}
            </span>
        }
    } else {
        let clean_short: String = clean_content.chars().take(100).collect();
        html! { <span>{clean_short}{if clean_content.chars().count() > 100 { "..." } else { "" }}</span> }
    }
}
