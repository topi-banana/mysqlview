use yew::prelude::*;

use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum ChipTone {
    Neutral,
    Primary,
    Success,
    Warning,
    Error,
}

impl ChipTone {
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => theme::CHIP_NEUTRAL,
            Self::Primary => theme::CHIP_PRIMARY,
            Self::Success => theme::CHIP_SUCCESS,
            Self::Warning => theme::CHIP_WARNING,
            Self::Error => theme::CHIP_ERROR,
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub label: AttrValue,
    #[prop_or(ChipTone::Neutral)]
    pub tone: ChipTone,
}

pub struct Chip;

impl Component for Chip {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        html! { <span class={p.tone.class()}>{ p.label.clone() }</span> }
    }
}
