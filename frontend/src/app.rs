use yew::prelude::*;
use yew_router::prelude::*;

use crate::components::nav::Nav;
use crate::router::{Route, switch};

pub struct App;

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                <div class="min-h-screen flex flex-col">
                    <Nav />
                    <main class="flex-1 w-full max-w-[1280px] mx-auto px-6 py-8">
                        <Switch<Route> render={switch} />
                    </main>
                </div>
            </BrowserRouter>
        }
    }
}
