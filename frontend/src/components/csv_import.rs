use mysqlview_types::CsvImportResponse;
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
    pub table: String,
    pub on_close: Callback<()>,
    pub on_done: Callback<()>,
}

pub enum Msg {
    Body(String),
    FilePicked(web_sys::File),
    FileLoaded(Result<String, String>),
    Submit,
    Done(Result<CsvImportResponse, ApiClientError>),
}

pub struct CsvImport {
    body: String,
    busy: bool,
    api_error: Option<ApiClientError>,
    response: Option<CsvImportResponse>,
    local_error: Option<String>,
}

impl Component for CsvImport {
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
                    self.local_error = Some("Paste a CSV body or pick a file first.".into());
                    return true;
                }
                self.busy = true;
                self.api_error = None;
                self.local_error = None;
                self.response = None;
                let db = ctx.props().db.clone();
                let table = ctx.props().table.clone();
                let body = self.body.clone();
                ctx.link().send_future(async move {
                    Msg::Done(api::import_table_csv(&db, &table, &body).await)
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
                title={AttrValue::from(format!("Import CSV into `{}`.`{}`", ctx.props().db, ctx.props().table))}
                on_close={close_cb}
                width_class={AttrValue::from("max-w-3xl")}
            >
                <div class="space-y-4">
                    <p class="text-xs text-text-secondary leading-relaxed">
                        { "First row = column names (must match the target table). Empty unquoted cell = NULL; quoted \"\" = empty string. Bytes use the prefix " }
                        <code class="font-mono">{ "b64:" }</code>
                        { ". Imports run row-by-row and stop at the first error." }
                    </p>
                    <label class="block space-y-1.5">
                        <span class="block text-[13px] font-medium text-text">{ "Pick a .csv file" }</span>
                        <input
                            type="file"
                            accept=".csv,text/csv"
                            class="text-sm"
                            onchange={on_file}
                        />
                    </label>
                    <label class="block space-y-1.5">
                        <span class="block text-[13px] font-medium text-text">{ "…or paste CSV body" }</span>
                        <textarea
                            class={format!("{} font-mono leading-relaxed", theme::INPUT)}
                            rows="10"
                            placeholder="id,name\n1,ada"
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
                            { Html::from(if self.busy { "Importing…" } else { "Import" }) }
                        </Button>
                    </div>
                </div>
            </Modal>
        }
    }
}

fn view_summary(resp: &CsvImportResponse) -> Html {
    match &resp.failed_at {
        None => html! {
            <div class="bg-success/10 border border-success/30 rounded-[12px] p-3 text-sm text-success">
                { format!("Inserted {} row(s).", resp.inserted) }
            </div>
        },
        Some(f) => html! {
            <div class="bg-error/5 border border-error/30 rounded-[12px] p-3 text-sm text-error space-y-1">
                <div class="font-medium">{ format!("Stopped at row {}", f.row_index) }</div>
                <div class="text-xs">{ &f.message }</div>
                <div class="text-xs text-text-secondary">
                    { format!("{} row(s) committed before the failure.", resp.inserted) }
                </div>
            </div>
        },
    }
}
