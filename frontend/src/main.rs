mod api;
mod app;
mod components;
mod pages;
mod router;
mod state;
mod theme;
mod theme_provider;
mod util;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
