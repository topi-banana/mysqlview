use mysqlview_types::TableSummary;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::empty_state::EmptyState;
use crate::components::error_banner::ErrorBanner;
use crate::components::input::TextInput;
use crate::components::skeleton::Skeleton;
use crate::router::Route;
use crate::state::LoadingState;
use crate::theme;

pub enum Msg {
    Fetch,
    Loaded(Result<Vec<TableSummary>, ApiClientError>),
    Filter(String),
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
}

pub struct DatabasePage {
    state: LoadingState<Vec<TableSummary>>,
    filter: String,
}

impl Component for DatabasePage {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Fetch);
        Self {
            state: LoadingState::Loading,
            filter: String::new(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old: &Self::Properties) -> bool {
        ctx.link().send_message(Msg::Fetch);
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Fetch => {
                self.state = LoadingState::Loading;
                let db = ctx.props().db.clone();
                ctx.link()
                    .send_future(async move { Msg::Loaded(api::list_tables(&db).await) });
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
            Msg::Filter(s) => {
                self.filter = s;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let db = &ctx.props().db;
        html! {
            <div class="space-y-6">
                <div class="space-y-2">
                    <div class={theme::OVERLINE}>{ "Database" }</div>
                    <h1 class={theme::SECTION_HEADING}>{ db }</h1>
                </div>
                <TextInput
                    placeholder="Filter tables…"
                    value={AttrValue::from(self.filter.clone())}
                    oninput={ctx.link().callback(Msg::Filter)}
                />
                { self.view_body(ctx) }
            </div>
        }
    }
}

impl DatabasePage {
    fn view_body(&self, ctx: &Context<Self>) -> Html {
        let db = &ctx.props().db;
        match &self.state {
            LoadingState::Idle | LoadingState::Loading => html! { <Skeleton rows={6} /> },
            LoadingState::Failed(e) => html! { <ErrorBanner error={e.clone()} /> },
            LoadingState::Ready(list) if list.is_empty() => html! {
                <EmptyState
                    title="No tables"
                    description="This database has no tables."
                />
            },
            LoadingState::Ready(list) => {
                let filter = self.filter.to_lowercase();
                let filtered: Vec<&TableSummary> = list
                    .iter()
                    .filter(|t| filter.is_empty() || t.name.to_lowercase().contains(&filter))
                    .collect();
                html! {
                    <div class="bg-surface border border-border rounded-[12px] overflow-hidden">
                        <table class="w-full border-collapse text-sm">
                            <thead class="border-b border-border">
                                <tr>
                                    <th class="text-left px-4 py-3 font-display font-semibold text-[13px]">{ "Table" }</th>
                                    <th class="text-left px-4 py-3 font-display font-semibold text-[13px]">{ "Engine" }</th>
                                    <th class="text-right px-4 py-3 font-display font-semibold text-[13px]">{ "Rows" }</th>
                                    <th class="text-right px-4 py-3 font-display font-semibold text-[13px]">{ "Size" }</th>
                                    <th class="text-right px-4 py-3"></th>
                                </tr>
                            </thead>
                            <tbody>
                                { for filtered.iter().map(|t| view_table_row(db, t)) }
                            </tbody>
                        </table>
                    </div>
                }
            }
        }
    }
}

fn view_table_row(db: &str, t: &TableSummary) -> Html {
    let browse_to = Route::Browse {
        db: db.to_owned(),
        table: t.name.clone(),
    };
    let structure_to = Route::Structure {
        db: db.to_owned(),
        table: t.name.clone(),
    };
    html! {
        <tr class="border-b border-border last:border-b-0 hover:bg-background/60">
            <td class="px-4 py-3 font-mono text-[13px] font-medium">{ &t.name }</td>
            <td class="px-4 py-3 text-text-secondary text-[13px]">
                { t.engine.clone().unwrap_or_default() }
            </td>
            <td class="px-4 py-3 text-right font-mono text-[13px]">
                { t.rows.map(|n| n.to_string()).unwrap_or_else(|| "—".into()) }
            </td>
            <td class="px-4 py-3 text-right font-mono text-[13px] text-text-secondary">
                { t.data_length.map(crate::theme::format_bytes).unwrap_or_else(|| "—".into()) }
            </td>
            <td class="px-4 py-3 text-right">
                <Link<Route> to={browse_to} classes="text-primary text-[13px] font-medium hover:underline mr-3">
                    { "Browse" }
                </Link<Route>>
                <Link<Route> to={structure_to} classes="text-primary text-[13px] font-medium hover:underline">
                    { "Structure" }
                </Link<Route>>
            </td>
        </tr>
    }
}
