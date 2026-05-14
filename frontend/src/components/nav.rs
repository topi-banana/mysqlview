use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::Route;

pub struct Nav;

impl Component for Nav {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <header class="sticky top-0 z-20 h-14 backdrop-blur-md bg-surface/80 border-b border-border">
                <div class="h-full max-w-[1280px] mx-auto px-6 flex items-center justify-between">
                    <Link<Route>
                        to={Route::Home}
                        classes="font-display text-lg font-semibold tracking-tight"
                    >
                        { "mysqlview" }
                    </Link<Route>>
                    <nav class="flex items-center gap-1">
                        <Link<Route>
                            to={Route::Home}
                            classes="px-3 py-1.5 rounded-[6px] text-sm font-medium text-text-secondary hover:text-text hover:bg-background"
                        >
                            { "Databases" }
                        </Link<Route>>
                        <Link<Route>
                            to={Route::Console}
                            classes="px-3 py-1.5 rounded-[6px] text-sm font-medium text-text-secondary hover:text-text hover:bg-background"
                        >
                            { "Console" }
                        </Link<Route>>
                    </nav>
                </div>
            </header>
        }
    }
}
