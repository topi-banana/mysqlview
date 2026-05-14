use mysqlview_types::{BrowseRequest, BrowseResponse, SortOrder};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::data_grid::DataGrid;
use crate::components::error_banner::ErrorBanner;
use crate::components::pagination::Pagination;
use crate::components::skeleton::Skeleton;
use crate::router::Route;
use crate::state::LoadingState;
use crate::theme;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
    pub table: String,
}

pub enum Msg {
    Fetch,
    Loaded(Result<BrowseResponse, ApiClientError>),
    ChangePage(u64),
    Sort(String),
}

pub struct TableBrowsePage {
    state: LoadingState<BrowseResponse>,
    request: BrowseRequest,
}

impl Component for TableBrowsePage {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Fetch);
        Self {
            state: LoadingState::Loading,
            request: BrowseRequest::default(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old: &Self::Properties) -> bool {
        self.request = BrowseRequest::default();
        ctx.link().send_message(Msg::Fetch);
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Fetch => {
                self.state = LoadingState::Loading;
                let p = ctx.props();
                let db = p.db.clone();
                let table = p.table.clone();
                let req = self.request.clone();
                ctx.link().send_future(async move {
                    Msg::Loaded(api::browse_rows(&db, &table, &req).await)
                });
                true
            }
            Msg::Loaded(Ok(r)) => {
                self.state = LoadingState::Ready(r);
                true
            }
            Msg::Loaded(Err(e)) => {
                self.state = LoadingState::Failed(e);
                true
            }
            Msg::ChangePage(offset) => {
                self.request.offset = offset;
                ctx.link().send_message(Msg::Fetch);
                false
            }
            Msg::Sort(column) => {
                if self.request.sort.as_deref() == Some(column.as_str()) {
                    self.request.order = Some(match self.request.order {
                        Some(SortOrder::Asc) => SortOrder::Desc,
                        _ => SortOrder::Asc,
                    });
                } else {
                    self.request.sort = Some(column);
                    self.request.order = Some(SortOrder::Asc);
                }
                self.request.offset = 0;
                ctx.link().send_message(Msg::Fetch);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        html! {
            <div class="space-y-6">
                <div class="space-y-1">
                    <div class={theme::OVERLINE}>
                        <Link<Route> to={Route::Database { db: p.db.clone() }} classes="hover:text-text">
                            { &p.db }
                        </Link<Route>>
                        { " · browse" }
                    </div>
                    <h1 class={theme::SECTION_HEADING}>{ &p.table }</h1>
                    <div class="flex gap-2 pt-2">
                        <Link<Route>
                            to={Route::Structure { db: p.db.clone(), table: p.table.clone() }}
                            classes={theme::BTN_GHOST}
                        >
                            { "Structure" }
                        </Link<Route>>
                    </div>
                </div>
                { self.view_body(ctx) }
            </div>
        }
    }
}

impl TableBrowsePage {
    fn view_body(&self, ctx: &Context<Self>) -> Html {
        match &self.state {
            LoadingState::Idle | LoadingState::Loading => html! { <Skeleton rows={8} /> },
            LoadingState::Failed(e) => html! { <ErrorBanner error={e.clone()} /> },
            LoadingState::Ready(resp) => {
                let on_sort = ctx.link().callback(Msg::Sort);
                let on_change = ctx.link().callback(Msg::ChangePage);
                html! {
                    <div class="space-y-3">
                        <Pagination
                            offset={self.request.offset}
                            limit={self.request.limit}
                            total={resp.total}
                            on_change={on_change}
                        />
                        <DataGrid
                            columns={resp.columns.clone()}
                            rows={resp.rows.clone()}
                            on_sort={on_sort}
                            sort_column={self.request.sort.clone()}
                        />
                        <div class="text-xs text-text-secondary text-right">
                            { format!("loaded in {} ms", resp.duration_ms) }
                        </div>
                    </div>
                }
            }
        }
    }
}
