use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or(3)]
    pub rows: u32,
    #[prop_or_default]
    pub label: AttrValue,
}

pub struct Skeleton;

impl Component for Skeleton {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        html! {
            <div class="space-y-3" aria-label={p.label.clone()}>
                { for (0..p.rows).map(|i| {
                    let width = match i % 3 {
                        0 => "w-full",
                        1 => "w-11/12",
                        _ => "w-3/4",
                    };
                    html! {
                        <div class={classes!("h-4", "rounded", "bg-border", "animate-pulse", width)}></div>
                    }
                }) }
            </div>
        }
    }
}
