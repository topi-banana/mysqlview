use mysqlview_types::{ColumnInfo, ForeignKeyInfo, IndexInfo, TableStructure};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::chip::{Chip, ChipTone};
use crate::components::error_banner::ErrorBanner;
use crate::components::skeleton::Skeleton;
use crate::router::Route;
use crate::state::LoadingState;
use crate::theme;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
    pub table: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Columns,
    Indexes,
    ForeignKeys,
    Create,
}

pub enum Msg {
    Fetch,
    Loaded(Result<TableStructure, ApiClientError>),
    SetTab(Tab),
}

pub struct TableStructurePage {
    state: LoadingState<TableStructure>,
    tab: Tab,
}

impl Component for TableStructurePage {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Fetch);
        Self {
            state: LoadingState::Loading,
            tab: Tab::Columns,
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
                let p = ctx.props();
                let db = p.db.clone();
                let table = p.table.clone();
                ctx.link().send_future(async move {
                    Msg::Loaded(api::describe_table(&db, &table).await)
                });
                true
            }
            Msg::Loaded(Ok(s)) => {
                self.state = LoadingState::Ready(s);
                true
            }
            Msg::Loaded(Err(e)) => {
                self.state = LoadingState::Failed(e);
                true
            }
            Msg::SetTab(t) => {
                self.tab = t;
                true
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
                        { " · table" }
                    </div>
                    <h1 class={theme::SECTION_HEADING}>{ &p.table }</h1>
                    <div class="flex gap-2 pt-2">
                        <Link<Route>
                            to={Route::Browse { db: p.db.clone(), table: p.table.clone() }}
                            classes={theme::BTN_GHOST}
                        >
                            { "Browse" }
                        </Link<Route>>
                    </div>
                </div>
                <div class="flex gap-1 border-b border-border">
                    { self.tab_button(ctx, Tab::Columns, "Columns") }
                    { self.tab_button(ctx, Tab::Indexes, "Indexes") }
                    { self.tab_button(ctx, Tab::ForeignKeys, "Foreign Keys") }
                    { self.tab_button(ctx, Tab::Create, "CREATE") }
                </div>
                { self.view_body() }
            </div>
        }
    }
}

impl TableStructurePage {
    fn tab_button(&self, ctx: &Context<Self>, tab: Tab, label: &str) -> Html {
        let active = self.tab == tab;
        let click = ctx.link().callback(move |_| Msg::SetTab(tab));
        let class = if active {
            "px-4 py-2 text-sm font-medium border-b-2 border-primary text-text"
        } else {
            "px-4 py-2 text-sm font-medium border-b-2 border-transparent text-text-secondary hover:text-text"
        };
        html! { <button class={class} onclick={click}>{ label }</button> }
    }

    fn view_body(&self) -> Html {
        match &self.state {
            LoadingState::Idle | LoadingState::Loading => html! { <Skeleton rows={6} /> },
            LoadingState::Failed(e) => html! { <ErrorBanner error={e.clone()} /> },
            LoadingState::Ready(s) => match self.tab {
                Tab::Columns => view_columns(&s.columns),
                Tab::Indexes => view_indexes(&s.indexes),
                Tab::ForeignKeys => view_foreign_keys(&s.foreign_keys),
                Tab::Create => view_create(&s.create_statement),
            },
        }
    }
}

fn view_columns(columns: &[ColumnInfo]) -> Html {
    html! {
        <div class="bg-surface border border-border rounded-[12px] overflow-hidden">
            <table class="w-full border-collapse text-sm">
                <thead class="border-b border-border">
                    <tr>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Name" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Type" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Null" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Key" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Default" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Extra" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for columns.iter().map(|c| html! {
                        <tr class="border-b border-border last:border-b-0">
                            <td class="px-4 py-2.5 font-mono text-[13px] font-medium">{ &c.name }</td>
                            <td class="px-4 py-2.5 font-mono text-[13px] text-text-secondary">{ &c.data_type }</td>
                            <td class="px-4 py-2.5">
                                <Chip
                                    label={AttrValue::from(if c.nullable { "YES" } else { "NO" })}
                                    tone={if c.nullable { ChipTone::Warning } else { ChipTone::Neutral }}
                                />
                            </td>
                            <td class="px-4 py-2.5 font-mono text-[13px]">{ c.key.clone().unwrap_or_default() }</td>
                            <td class="px-4 py-2.5 font-mono text-[13px] text-text-secondary">{ c.default.clone().unwrap_or_else(|| "—".into()) }</td>
                            <td class="px-4 py-2.5 font-mono text-[12px] text-text-secondary">{ c.extra.clone().unwrap_or_default() }</td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </div>
    }
}

fn view_indexes(indexes: &[IndexInfo]) -> Html {
    if indexes.is_empty() {
        return html! { <p class="text-text-secondary text-sm">{ "No indexes." }</p> };
    }
    html! {
        <div class="bg-surface border border-border rounded-[12px] overflow-hidden">
            <table class="w-full border-collapse text-sm">
                <thead class="border-b border-border">
                    <tr>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Name" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Columns" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Unique" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Primary" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for indexes.iter().map(|i| html! {
                        <tr class="border-b border-border last:border-b-0">
                            <td class="px-4 py-2.5 font-mono text-[13px] font-medium">{ &i.name }</td>
                            <td class="px-4 py-2.5 font-mono text-[13px] text-text-secondary">{ i.columns.join(", ") }</td>
                            <td class="px-4 py-2.5 text-[13px]">{ if i.unique { "✓" } else { "" } }</td>
                            <td class="px-4 py-2.5 text-[13px]">{ if i.primary { "✓" } else { "" } }</td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </div>
    }
}

fn view_foreign_keys(fks: &[ForeignKeyInfo]) -> Html {
    if fks.is_empty() {
        return html! { <p class="text-text-secondary text-sm">{ "No foreign keys." }</p> };
    }
    html! {
        <div class="bg-surface border border-border rounded-[12px] overflow-hidden">
            <table class="w-full border-collapse text-sm">
                <thead class="border-b border-border">
                    <tr>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Name" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "Columns" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "References" }</th>
                        <th class="text-left px-4 py-3 font-display text-[13px] font-semibold">{ "On Update / On Delete" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for fks.iter().map(|fk| html! {
                        <tr class="border-b border-border last:border-b-0">
                            <td class="px-4 py-2.5 font-mono text-[13px] font-medium">{ &fk.name }</td>
                            <td class="px-4 py-2.5 font-mono text-[13px] text-text-secondary">{ fk.columns.join(", ") }</td>
                            <td class="px-4 py-2.5 font-mono text-[13px] text-text-secondary">
                                { format!("{}({})", fk.ref_table, fk.ref_columns.join(", ")) }
                            </td>
                            <td class="px-4 py-2.5 font-mono text-[12px] text-text-secondary">
                                { format!("{} / {}", fk.on_update.clone().unwrap_or_else(|| "—".into()), fk.on_delete.clone().unwrap_or_else(|| "—".into())) }
                            </td>
                        </tr>
                    }) }
                </tbody>
            </table>
        </div>
    }
}

fn view_create(create: &str) -> Html {
    html! {
        <pre class="bg-surface border border-border rounded-[12px] p-5 text-[13px] font-mono overflow-x-auto whitespace-pre">
            { create }
        </pre>
    }
}
