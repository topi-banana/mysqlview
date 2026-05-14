use yew::prelude::*;

use crate::components::button::{Button, ButtonVariant};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub offset: u64,
    pub limit: u32,
    pub total: Option<u64>,
    pub on_change: Callback<u64>,
}

pub struct Pagination;

impl Component for Pagination {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let page = p.offset / p.limit.max(1) as u64 + 1;
        let limit = p.limit.max(1) as u64;
        let total_pages = p.total.map(|t| t.div_ceil(limit));
        let on_change_prev = p.on_change.clone();
        let prev_offset = p.offset.saturating_sub(p.limit as u64);
        let prev_disabled = p.offset == 0;
        let on_prev = Callback::from(move |_| on_change_prev.emit(prev_offset));

        let on_change_next = p.on_change.clone();
        let next_offset = p.offset + p.limit as u64;
        let next_disabled = match p.total {
            Some(t) => next_offset >= t,
            None => false,
        };
        let on_next = Callback::from(move |_| on_change_next.emit(next_offset));

        let label = match total_pages {
            Some(t) => format!("Page {page} of {t}"),
            None => format!("Page {page}"),
        };

        html! {
            <div class="flex items-center justify-between gap-4 py-3">
                <span class="text-sm text-text-secondary">{ label }</span>
                <div class="flex items-center gap-2">
                    <Button
                        variant={ButtonVariant::Secondary}
                        disabled={prev_disabled}
                        onclick={on_prev}
                    >
                        { "← Previous" }
                    </Button>
                    <Button
                        variant={ButtonVariant::Secondary}
                        disabled={next_disabled}
                        onclick={on_next}
                    >
                        { "Next →" }
                    </Button>
                </div>
            </div>
        }
    }
}
