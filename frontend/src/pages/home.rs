use mysqlview_types::DatabaseSummary;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::card::Card;
use crate::components::empty_state::EmptyState;
use crate::components::error_banner::ErrorBanner;
use crate::components::skeleton::Skeleton;
use crate::router::Route;
use crate::state::LoadingState;
use crate::theme;

pub enum Msg {
    Fetch,
    Loaded(Result<Vec<DatabaseSummary>, ApiClientError>),
}

pub struct HomePage {
    state: LoadingState<Vec<DatabaseSummary>>,
}

impl Component for HomePage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Fetch);
        Self {
            state: LoadingState::Loading,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Fetch => {
                self.state = LoadingState::Loading;
                ctx.link()
                    .send_future(async { Msg::Loaded(api::list_databases().await) });
                true
            }
            Msg::Loaded(Ok(list)) => {
                self.state = LoadingState::Ready(list);
                true
            }
            Msg::Loaded(Err(e)) => {
                self.state = LoadingState::Failed(e);
                true
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="space-y-6">
                <div class="space-y-2">
                    <h1 class={theme::SECTION_HEADING}>{ "Databases" }</h1>
                    <p class="text-text-secondary text-sm">
                        { "Schemas available on the connected MySQL server." }
                    </p>
                </div>
                { self.view_body() }
            </div>
        }
    }
}

impl HomePage {
    fn view_body(&self) -> Html {
        match &self.state {
            LoadingState::Idle | LoadingState::Loading => html! { <Skeleton rows={4} /> },
            LoadingState::Failed(e) => html! { <ErrorBanner error={e.clone()} /> },
            LoadingState::Ready(list) if list.is_empty() => html! {
                <EmptyState
                    title="No databases"
                    description="The MySQL server has no user databases (system schemas are hidden)."
                />
            },
            LoadingState::Ready(list) => html! {
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
                    { for list.iter().map(view_db_card) }
                </div>
            },
        }
    }
}

fn view_db_card(db: &DatabaseSummary) -> Html {
    let name = db.name.clone();
    html! {
        <Link<Route> to={Route::Database { db: name.clone() }} classes="block">
            <Card interactive=true>
                <div class="p-5 space-y-3">
                    <div class={theme::OVERLINE}>{ "Schema" }</div>
                    <div class="font-display text-xl font-semibold tracking-tight">{ &db.name }</div>
                    <div class="flex flex-wrap gap-3 text-xs text-text-secondary font-mono">
                        if let Some(charset) = &db.charset {
                            <span>{ charset }</span>
                        }
                        if let Some(collation) = &db.collation {
                            <span>{ collation }</span>
                        }
                    </div>
                </div>
            </Card>
        </Link<Route>>
    }
}
