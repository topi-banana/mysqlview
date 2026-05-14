use yew::prelude::*;

use crate::api::ApiClientError;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub error: ApiClientError,
}

pub struct ErrorBanner;

impl Component for ErrorBanner {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let err = &ctx.props().error;
        html! {
            <div class="bg-error/5 border border-error/30 rounded-[12px] p-4 space-y-1">
                <div class="font-display font-semibold text-error text-sm">{ "Error" }</div>
                <div class="text-sm text-text">{ err.user_message() }</div>
                if let Some(hint) = err.hint() {
                    <div class="text-xs text-text-secondary font-mono">{ hint }</div>
                }
            </div>
        }
    }
}
