use yew::prelude::*;

use crate::theme;

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub children: Children,
    #[prop_or(false)]
    pub interactive: bool,
    #[prop_or_default]
    pub class: AttrValue,
}

pub struct Card;

impl Component for Card {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let base = if p.interactive {
            theme::CARD
        } else {
            theme::CARD_FLAT
        };
        let class = if p.class.is_empty() {
            base.to_owned()
        } else {
            format!("{base} {}", p.class)
        };
        html! { <div {class}>{ for p.children.iter() }</div> }
    }
}
