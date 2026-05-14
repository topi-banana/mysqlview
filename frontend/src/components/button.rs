use yew::prelude::*;

use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
}

impl ButtonVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Primary => theme::BTN_PRIMARY,
            Self::Secondary => theme::BTN_SECONDARY,
            Self::Ghost => theme::BTN_GHOST,
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub children: Children,
    #[prop_or(ButtonVariant::Secondary)]
    pub variant: ButtonVariant,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or_default]
    pub onclick: Callback<MouseEvent>,
    #[prop_or_default]
    pub type_: AttrValue,
}

pub struct Button;

impl Component for Button {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let onclick = p.onclick.clone();
        let kind = if p.type_.is_empty() {
            "button".into()
        } else {
            p.type_.clone()
        };
        html! {
            <button
                class={p.variant.class()}
                type={kind}
                disabled={p.disabled}
                onclick={Callback::from(move |e: MouseEvent| onclick.emit(e))}
            >
                { for p.children.iter() }
            </button>
        }
    }
}
