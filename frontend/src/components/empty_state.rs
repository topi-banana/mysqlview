use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub title: AttrValue,
    #[prop_or_default]
    pub description: AttrValue,
}

pub struct EmptyState;

impl Component for EmptyState {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        html! {
            <div class="bg-surface border border-border rounded-[12px] p-10 text-center">
                <h3 class="font-display text-lg font-semibold tracking-tight mb-2">{ p.title.clone() }</h3>
                if !p.description.is_empty() {
                    <p class="text-sm text-text-secondary">{ p.description.clone() }</p>
                }
            </div>
        }
    }
}
