use mysqlview_types::SqlImportResponse;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::button::{Button, ButtonVariant};
use crate::components::error_banner::ErrorBanner;
use crate::components::modal::Modal;
use crate::theme;
use crate::util::download::read_file_as_text;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
    pub on_close: Callback<()>,
    pub on_done: Callback<()>,
}

pub enum Msg {
    Body(String),
    FilePicked(web_sys::File),
    FileLoaded(Result<String, String>),
    Submit,
    Done(Result<SqlImportResponse, ApiClientError>),
}

pub struct SqlImport {
    body: String,
    busy: bool,
    api_error: Option<ApiClientError>,
    response: Option<SqlImportResponse>,
    local_error: Option<String>,
}

impl Component for SqlImport {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            body: String::new(),
            busy: false,
            api_error: None,
            response: None,
            local_error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Body(s) => {
                self.body = s;
                true
            }
            Msg::FilePicked(file) => {
                self.local_error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    link.send_message(Msg::FileLoaded(
                        read_file_as_text(&file).await.map_err(|e| e.to_string()),
                    ));
                });
                false
            }
            Msg::FileLoaded(Ok(text)) => {
                self.body = text;
                true
            }
            Msg::FileLoaded(Err(e)) => {
                self.local_error = Some(format!("Could not read file: {e}"));
                true
            }
            Msg::Submit => {
                if self.body.trim().is_empty() {
                    self.local_error = Some("Paste a SQL script or pick a file first.".into());
                    return true;
                }
                self.busy = true;
                self.api_error = None;
                self.local_error = None;
                self.response = None;
                let db = ctx.props().db.clone();
                let body = self.body.clone();
                ctx.link().send_future(async move {
                    Msg::Done(api::import_database_sql(&db, &body).await)
                });
                true
            }
            Msg::Done(Ok(resp)) => {
                self.busy = false;
                self.response = Some(resp.clone());
                if resp.failed_at.is_none() {
                    ctx.props().on_done.emit(());
                }
                true
            }
            Msg::Done(Err(e)) => {
                self.busy = false;
                self.api_error = Some(e);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let close = ctx.props().on_close.clone();
        let close_cb = Callback::from(move |_| close.emit(()));
        let cancel = ctx.props().on_close.clone();
        let cancel_cb = Callback::from(move |_| cancel.emit(()));
        let submit = ctx.link().callback(|_| Msg::Submit);
        let on_file = ctx.link().callback(|e: Event| {
            let file = e
                .target()
                .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                .and_then(|i| i.files())
                .and_then(|fl| fl.item(0));
            match file {
                Some(f) => Msg::FilePicked(f),
                None => Msg::FileLoaded(Err("no file selected".into())),
            }
        });
        let on_textarea = ctx.link().callback(|e: InputEvent| {
            let v = e
                .target()
                .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|t| t.value())
                .unwrap_or_default();
            Msg::Body(v)
        });

        html! {
            <Modal
                title={AttrValue::from(format!("Run SQL against `{}`", ctx.props().db))}
                on_close={close_cb}
                width_class={AttrValue::from("max-w-3xl")}
            >
                <div class="space-y-4">
                    <p class="text-xs text-text-secondary leading-relaxed">
                        { "Statements run one at a time against the selected database. The import stops at the first failure — DDL (CREATE/ALTER/DROP) implicitly commits in MySQL, so partial state may remain after an error. " }
                        <code class="font-mono">{ "DELIMITER" }</code>
                        { " directives are not supported." }
                    </p>
                    <label class="block space-y-1.5">
                        <span class="block text-[13px] font-medium text-text">{ "Pick a .sql file" }</span>
                        <input
                            type="file"
                            accept=".sql,application/sql,text/plain"
                            class="text-sm"
                            onchange={on_file}
                        />
                    </label>
                    <label class="block space-y-1.5">
                        <span class="block text-[13px] font-medium text-text">{ "…or paste a SQL script" }</span>
                        <textarea
                            class={format!("{} font-mono leading-relaxed", theme::INPUT)}
                            rows="12"
                            placeholder="-- run any number of statements separated by ;"
                            value={self.body.clone()}
                            oninput={on_textarea}
                        />
                    </label>

                    if let Some(msg) = &self.local_error {
                        <div class="bg-error/5 border border-error/30 rounded-[12px] p-3 text-sm text-error">
                            { msg }
                        </div>
                    }
                    if let Some(e) = &self.api_error {
                        <ErrorBanner error={e.clone()} />
                    }
                    if let Some(resp) = &self.response {
                        { view_summary(resp) }
                    }

                    <div class="flex justify-end gap-2 pt-2">
                        <Button
                            variant={ButtonVariant::Secondary}
                            disabled={self.busy}
                            onclick={cancel_cb}
                        >
                            { Html::from(if self.response.is_some() { "Close" } else { "Cancel" }) }
                        </Button>
                        <Button
                            variant={ButtonVariant::Primary}
                            disabled={self.busy || self.body.trim().is_empty()}
                            onclick={submit}
                        >
                            { Html::from(if self.busy { "Running…" } else { "Run" }) }
                        </Button>
                    </div>
                </div>
            </Modal>
        }
    }
}

fn view_summary(resp: &SqlImportResponse) -> Html {
    match &resp.failed_at {
        None => html! {
            <div class="bg-success/10 border border-success/30 rounded-[12px] p-3 text-sm text-success">
                { format!(
                    "Ran {} statement(s); {} row(s) affected.",
                    resp.statements_run, resp.total_affected_rows
                ) }
            </div>
        },
        Some(f) => html! {
            <div class="bg-error/5 border border-error/30 rounded-[12px] p-3 text-sm text-error space-y-1">
                <div class="font-medium">{ format!("Stopped at statement {}", f.statement_index) }</div>
                <pre class="text-xs font-mono whitespace-pre-wrap">{ &f.statement_preview }</pre>
                <div class="text-xs">{ &f.message }</div>
                <div class="text-xs text-text-secondary">
                    { format!(
                        "{} statement(s) committed before the failure; {} row(s) affected.",
                        resp.statements_run, resp.total_affected_rows
                    ) }
                </div>
            </div>
        },
    }
}
