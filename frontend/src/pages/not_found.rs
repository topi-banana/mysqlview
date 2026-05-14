use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::Route;
use crate::theme;

pub struct NotFoundPage;

impl Component for NotFoundPage {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="py-16 text-center space-y-4">
                <h1 class="font-display text-5xl font-semibold tracking-tight">{ "404" }</h1>
                <p class="text-text-secondary">{ "This page does not exist." }</p>
                <Link<Route> to={Route::Home} classes={theme::BTN_PRIMARY}>{ "Back to home" }</Link<Route>>
            </div>
        }
    }
}
