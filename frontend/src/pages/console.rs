use mysqlview_types::{QueryRequest, QueryResponse};
use yew::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::button::{Button, ButtonVariant};
use crate::components::code_editor::CodeEditor;
use crate::components::data_grid::DataGrid;
use crate::components::error_banner::ErrorBanner;
use crate::components::skeleton::Skeleton;
use crate::state::LoadingState;
use crate::theme;

pub enum Msg {
    Edit(String),
    Run,
    Result(Result<QueryResponse, ApiClientError>),
}

pub struct ConsolePage {
    sql: String,
    state: LoadingState<QueryResponse>,
}

impl Component for ConsolePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            sql: String::new(),
            state: LoadingState::Idle,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Edit(s) => {
                self.sql = s;
                false
            }
            Msg::Run => {
                let sql = self.sql.trim().to_string();
                if sql.is_empty() {
                    return false;
                }
                self.state = LoadingState::Loading;
                ctx.link().send_future(async move {
                    Msg::Result(api::run_query(&QueryRequest { sql }).await)
                });
                true
            }
            Msg::Result(Ok(r)) => {
                self.state = LoadingState::Ready(r);
                true
            }
            Msg::Result(Err(e)) => {
                self.state = LoadingState::Failed(e);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().callback(Msg::Edit);
        let onclick = ctx.link().callback(|_| Msg::Run);
        let running = self.state.is_loading();
        let onkeydown = ctx.link().batch_callback(|e: KeyboardEvent| {
            if (e.meta_key() || e.ctrl_key()) && e.key() == "Enter" {
                e.prevent_default();
                Some(Msg::Run)
            } else {
                None
            }
        });
        html! {
            <div class="space-y-5">
                <div class="space-y-2">
                    <h1 class={theme::SECTION_HEADING}>{ "Console" }</h1>
                    <p class="text-sm text-text-secondary">
                        { "Run arbitrary SQL against the connected MySQL server. " }
                        <span class="font-mono">{ "⌘/Ctrl + Enter" }</span>
                        { " to execute." }
                    </p>
                </div>
                <CodeEditor
                    value={AttrValue::from(self.sql.clone())}
                    placeholder="SELECT * FROM information_schema.tables LIMIT 10"
                    oninput={oninput}
                    onkeydown={onkeydown}
                    rows={10}
                />
                <div>
                    <Button variant={ButtonVariant::Primary} disabled={running} onclick={onclick}>
                        { Html::from(if running { "Running…" } else { "Run" }) }
                    </Button>
                </div>
                { self.view_result() }
            </div>
        }
    }
}

impl ConsolePage {
    fn view_result(&self) -> Html {
        match &self.state {
            LoadingState::Idle => Html::default(),
            LoadingState::Loading => html! { <Skeleton rows={6} /> },
            LoadingState::Failed(e) => html! { <ErrorBanner error={e.clone()} /> },
            LoadingState::Ready(QueryResponse::ResultSet {
                columns,
                rows,
                duration_ms,
                truncated,
            }) => html! {
                <div class="space-y-3">
                    <div class="flex items-center gap-3 text-xs text-text-secondary">
                        <span>{ format!("{} rows", rows.len()) }</span>
                        <span>{ format!("· {duration_ms} ms") }</span>
                        if *truncated {
                            <span class="text-warning">{ "· result truncated" }</span>
                        }
                    </div>
                    <DataGrid columns={columns.clone()} rows={rows.clone()} />
                </div>
            },
            LoadingState::Ready(QueryResponse::Affected {
                affected_rows,
                last_insert_id,
                duration_ms,
                warnings,
            }) => html! {
                <div class="bg-surface border border-border rounded-[12px] p-5 space-y-2">
                    <div class="font-display text-lg font-semibold tracking-tight">
                        { format!("{affected_rows} row(s) affected") }
                    </div>
                    <div class="text-xs text-text-secondary space-x-3">
                        <span>{ format!("{duration_ms} ms") }</span>
                        if let Some(id) = last_insert_id {
                            <span>{ format!("· last_insert_id = {id}") }</span>
                        }
                    </div>
                    if !warnings.is_empty() {
                        <ul class="text-xs text-warning list-disc list-inside">
                            { for warnings.iter().map(|w| html!{ <li>{ w }</li> }) }
                        </ul>
                    }
                </div>
            },
        }
    }
}
