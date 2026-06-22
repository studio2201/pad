use yew::prelude::*;
use pulldown_cmark::{Parser, Options, html};
use wasm_bindgen::JsCast;

#[derive(Properties, PartialEq)]
pub struct PreviewProps {
    pub content: String,
    pub is_visible: bool,
}

const NOTE_SVG: &str = "<svg class=\"octicon octicon-info mr-2\" viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\" aria-hidden=\"true\" fill=\"currentColor\"><path d=\"M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-5a.75.75 0 0 0-.75.75v4.5a.75.75 0 0 0 1.5 0v-4.5A.75.75 0 0 0 8 3ZM6 10a1 1 0 1 1 2 0 1 1 0 0 1-2 0Z\"></path></svg>";
const TIP_SVG: &str = "<svg class=\"octicon octicon-light-bulb mr-2\" viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\" aria-hidden=\"true\" fill=\"currentColor\"><path d=\"M8 1.5c-2.363 0-4 1.837-4 4 0 .963.351 1.763 1 2.375a.75.75 0 0 1 .277.562v1.313c0 .414.336.75.75.75h4a.75.75 0 0 0 .75-.75V8.438a.75.75 0 0 1 .277-.562c.65-.612 1-1.412 1-2.375 0-2.163-1.637-4-4-4Zm0 11a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z\"></path></svg>";
const IMPORTANT_SVG: &str = "<svg class=\"octicon octicon-report mr-2\" viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\" aria-hidden=\"true\" fill=\"currentColor\"><path d=\"M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v12.5A1.75 1.75 0 0 1 14.25 16H1.75A1.75 1.75 0 0 1 0 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25Zm6.25 3.5a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0ZM6.75 7.25h2.5a.75.75 0 0 1 0 1.5H8v2.5a.75.75 0 0 1-1.5 0v-3.25a.75.75 0 0 1 .25-.5Z\"></path></svg>";
const WARNING_SVG: &str = "<svg class=\"octicon octicon-alert mr-2\" viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\" aria-hidden=\"true\" fill=\"currentColor\"><path d=\"M6.457 1.047a2.25 2.25 0 0 1 3.086 0l5.436 5.436a2.25 2.25 0 0 1 0 3.086l-5.436 5.436a2.25 2.25 0 0 1-3.086 0L1.021 9.569a2.25 2.25 0 0 1 0-3.086Zm1.153 1.153a.75.75 0 0 0-1.06 0L1.114 7.636a.75.75 0 0 0 0 1.06l5.436 5.436a.75.75 0 0 0 1.06 0l5.436-5.436a.75.75 0 0 0 0-1.06L7.61 2.2Zm.39 3.55a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5a.75.75 0 0 1 .75-.75Zm0 7a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z\"></path></svg>";
const CAUTION_SVG: &str = "<svg class=\"octicon octicon-stop mr-2\" viewBox=\"0 0 16 16\" version=\"1.1\" width=\"16\" height=\"16\" aria-hidden=\"true\" fill=\"currentColor\"><path d=\"M4.47 .22A.75.75 0 0 1 5 0h6a.75.75 0 0 1 .53.22l4.25 4.25c.141.14.22.331.22.53v6a.75.75 0 0 1-.22.53l-4.25 4.25A.75.75 0 0 1 11 16H5a.75.75 0 0 1-.53-.22L.22 11.53A.75.75 0 0 1 0 11V5a.75.75 0 0 1 .22-.53Zm.83 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5H5.3Z\"></path></svg>";

#[wasm_bindgen::prelude::wasm_bindgen(inline_js = "
    export function sanitize_html(html) {
        const doc = new DOMParser().parseFromString(html, 'text/html');
        const clean = (node) => {
            const kids = Array.from(node.childNodes);
            for (const kid of kids) {
                if (kid.nodeType === 1) {
                    const tag = kid.tagName.toLowerCase();
                    if (['script', 'iframe', 'object', 'embed', 'link', 'style', 'meta', 'base'].includes(tag)) {
                        kid.remove();
                    } else {
                        const attrs = Array.from(kid.attributes);
                        for (const attr of attrs) {
                            const name = attr.name.toLowerCase();
                            if (name.startsWith('on')) {
                                kid.removeAttribute(attr.name);
                            } else if (['href', 'src', 'action'].includes(name)) {
                                const val = attr.value.trim().toLowerCase();
                                if (val.startsWith('javascript:') || val.startsWith('data:')) {
                                    kid.removeAttribute(attr.name);
                                }
                            }
                        }
                        clean(kid);
                    }
                }
            }
        };
        clean(doc.body);
        return doc.body.innerHTML;
    }
")]
extern "C" {
    fn sanitize_html(html: &str) -> String;
}

pub fn parse_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let mut processed = html_output;
    let alerts = [
        ("NOTE", "note", NOTE_SVG),
        ("TIP", "tip", TIP_SVG),
        ("IMPORTANT", "important", IMPORTANT_SVG),
        ("WARNING", "warning", WARNING_SVG),
        ("CAUTION", "caution", CAUTION_SVG),
    ];

    for &(alert_key, alert_class, svg) in alerts.iter() {
        let target_br = format!("<blockquote>\n<p>[!{}]<br />", alert_key);
        let replacement = format!(
            "<div class=\"markdown-alert markdown-alert-{}\"><p class=\"markdown-alert-title\">{} {}</p><p>",
            alert_class, svg, alert_key.to_lowercase()
        );
        processed = processed.replace(&target_br, &replacement);

        let target_nl = format!("<blockquote>\n<p>[!{}]", alert_key);
        processed = processed.replace(&target_nl, &replacement);
    }

    processed = processed.replace("</p>\n</blockquote>", "</p>\n</div>");
    sanitize_html(&processed)
}

#[function_component(Preview)]
pub fn preview(props: &PreviewProps) -> Html {
    let content = props.content.clone();

    use_effect_with(content, |_| {
        let handle = gloo_timers::callback::Timeout::new(50, move || {
            if let Some(w) = web_sys::window() {
                if let Ok(hljs) = js_sys::Reflect::get(&w, &"hljs".into()) {
                    if !hljs.is_undefined() && !hljs.is_null() {
                        if let Ok(highlight_all) = js_sys::Reflect::get(&hljs, &"highlightAll".into()) {
                            if let Some(func) = highlight_all.dyn_ref::<js_sys::Function>() {
                                let _ = func.call0(&hljs);
                            }
                        }
                    }
                }
            }
        });
        move || drop(handle)
    });

    if !props.is_visible {
        return html! {};
    }

    let parsed_html = parse_markdown(&props.content);

    html! {
        <div id="preview-container" class="preview-container" style="display: block;">
            <div id="preview-pane">
                { Html::from_html_unchecked(AttrValue::from(parsed_html)) }
            </div>
        </div>
    }
}
