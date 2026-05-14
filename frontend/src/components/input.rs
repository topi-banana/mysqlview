use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::theme;

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub value: AttrValue,
    #[prop_or_default]
    pub placeholder: AttrValue,
    #[prop_or_default]
    pub oninput: Callback<String>,
    #[prop_or_default]
    pub onkeydown: Callback<KeyboardEvent>,
    #[prop_or_default]
    pub kind: AttrValue,
}

pub struct TextInput;

impl Component for TextInput {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let oninput_cb = p.oninput.clone();
        let oninput = Callback::from(move |e: InputEvent| {
            if let Some(input) = e
                .target()
                .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
            {
                oninput_cb.emit(input.value());
            }
        });
        let onkeydown = p.onkeydown.clone();
        let type_attr = if p.kind.is_empty() {
            "text".into()
        } else {
            p.kind.clone()
        };
        html! {
            <input
                class={theme::INPUT}
                type={type_attr}
                value={p.value.clone()}
                placeholder={p.placeholder.clone()}
                oninput={oninput}
                onkeydown={Callback::from(move |e| onkeydown.emit(e))}
            />
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct TextAreaProps {
    #[prop_or_default]
    pub value: AttrValue,
    #[prop_or_default]
    pub placeholder: AttrValue,
    #[prop_or_default]
    pub oninput: Callback<String>,
    #[prop_or_default]
    pub onkeydown: Callback<KeyboardEvent>,
    #[prop_or(6)]
    pub rows: u32,
    #[prop_or(false)]
    pub mono: bool,
}

pub struct TextArea;

impl Component for TextArea {
    type Message = ();
    type Properties = TextAreaProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let oninput_cb = p.oninput.clone();
        let oninput = Callback::from(move |e: InputEvent| {
            if let Some(input) = e
                .target()
                .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
            {
                oninput_cb.emit(input.value());
            }
        });
        let onkeydown = p.onkeydown.clone();
        let class = if p.mono {
            format!("{} font-mono leading-relaxed", theme::INPUT)
        } else {
            theme::INPUT.into()
        };
        html! {
            <textarea
                class={class}
                placeholder={p.placeholder.clone()}
                rows={p.rows.to_string()}
                value={p.value.clone()}
                oninput={oninput}
                onkeydown={Callback::from(move |e| onkeydown.emit(e))}
            />
        }
    }
}
