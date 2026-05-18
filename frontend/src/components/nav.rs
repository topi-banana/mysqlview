use yew::context::ContextHandle;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::Route;
use crate::theme_provider::{ThemeCtx, ThemePref};

pub enum Msg {
    Ctx(ThemeCtx),
}

pub struct Nav {
    theme_ctx: Option<ThemeCtx>,
    _handle: Option<ContextHandle<ThemeCtx>>,
}

impl Component for Nav {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let cb = ctx.link().callback(Msg::Ctx);
        let (initial, handle) = match ctx.link().context::<ThemeCtx>(cb) {
            Some((v, h)) => (Some(v), Some(h)),
            None => (None, None),
        };
        Self {
            theme_ctx: initial,
            _handle: handle,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, Msg::Ctx(c): Msg) -> bool {
        self.theme_ctx = Some(c);
        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let toggle = self
            .theme_ctx
            .as_ref()
            .map(view_theme_toggle)
            .unwrap_or_default();

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
                        { toggle }
                    </nav>
                </div>
            </header>
        }
    }
}

fn view_theme_toggle(ctx: &ThemeCtx) -> Html {
    let pref = ctx.pref;
    let onclick = {
        let cycle = ctx.cycle.clone();
        Callback::from(move |_| cycle.emit(()))
    };
    let aria = match pref {
        ThemePref::Light => "Currently light. Click for dark.",
        ThemePref::Dark => "Currently dark. Click to follow system.",
        ThemePref::System => "Following system. Click for light.",
    };
    let icon = match pref.next() {
        ThemePref::Light => sun_svg(),
        ThemePref::Dark => moon_svg(),
        ThemePref::System => monitor_svg(),
    };
    html! {
        <button
            type="button"
            onclick={onclick}
            aria-label={aria}
            title={aria}
            class="px-3 py-1.5 rounded-[6px] text-text-secondary hover:text-text hover:bg-background transition-colors flex items-center"
        >
            { icon }
        </button>
    }
}

fn sun_svg() -> Html {
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="4"/>
            <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>
        </svg>
    }
}

fn moon_svg() -> Html {
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
    }
}

fn monitor_svg() -> Html {
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
             fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
            <line x1="8" y1="21" x2="16" y2="21"/>
            <line x1="12" y1="17" x2="12" y2="21"/>
        </svg>
    }
}
