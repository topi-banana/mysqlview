use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub title: AttrValue,
    pub on_close: Callback<()>,
    #[prop_or_default]
    pub children: Children,
    #[prop_or(AttrValue::from("max-w-2xl"))]
    pub width_class: AttrValue,
}

pub struct Modal;

impl Component for Modal {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let on_close_bg = p.on_close.clone();
        let on_close_btn = p.on_close.clone();
        let backdrop = Callback::from(move |_| on_close_bg.emit(()));
        let close = Callback::from(move |_| on_close_btn.emit(()));
        let stop = Callback::from(|e: MouseEvent| e.stop_propagation());

        let panel_class = format!(
            "relative w-full {} bg-surface rounded-[12px] shadow-[0_20px_60px_rgba(0,0,0,0.18)] border border-border",
            p.width_class
        );

        html! {
            <div
                class="fixed inset-0 z-50 bg-black/30 backdrop-blur-sm flex items-start justify-center pt-20 px-4 overflow-y-auto"
                onclick={backdrop}
            >
                <div class={panel_class} onclick={stop}>
                    <div class="flex items-center justify-between px-6 py-4 border-b border-border">
                        <h2 class="font-display text-xl font-semibold tracking-tight">
                            { p.title.clone() }
                        </h2>
                        <button
                            class="text-text-secondary hover:text-text text-2xl leading-none px-2"
                            type="button"
                            aria-label="Close"
                            onclick={close}
                        >
                            { "×" }
                        </button>
                    </div>
                    <div class="p-6">
                        { for p.children.iter() }
                    </div>
                </div>
            </div>
        }
    }
}
