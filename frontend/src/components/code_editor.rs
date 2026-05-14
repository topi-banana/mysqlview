use yew::prelude::*;

use crate::components::input::{TextArea, TextAreaProps};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub value: AttrValue,
    pub oninput: Callback<String>,
    #[prop_or_default]
    pub onkeydown: Callback<KeyboardEvent>,
    #[prop_or(10)]
    pub rows: u32,
    #[prop_or_default]
    pub placeholder: AttrValue,
}

pub struct CodeEditor;

impl Component for CodeEditor {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let props = TextAreaProps {
            value: p.value.clone(),
            placeholder: p.placeholder.clone(),
            oninput: p.oninput.clone(),
            onkeydown: p.onkeydown.clone(),
            rows: p.rows,
            mono: true,
        };
        html! { <TextArea ..props /> }
    }
}
