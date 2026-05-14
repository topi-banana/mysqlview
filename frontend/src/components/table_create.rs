use mysqlview_types::{ColumnDefinition, CreateTableRequest};
use yew::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::button::{Button, ButtonVariant};
use crate::components::error_banner::ErrorBanner;
use crate::components::input::TextInput;
use crate::components::modal::Modal;
use crate::theme;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
    pub on_close: Callback<()>,
    pub on_created: Callback<String>,
}

#[derive(Clone, Default)]
struct ColumnDraft {
    name: String,
    data_type: String,
    nullable: bool,
    default: String,
    auto_increment: bool,
    is_pk: bool,
}

impl ColumnDraft {
    fn starter_pk() -> Self {
        Self {
            name: "id".into(),
            data_type: "INT UNSIGNED".into(),
            nullable: false,
            default: String::new(),
            auto_increment: true,
            is_pk: true,
        }
    }
}

pub enum Msg {
    TableName(String),
    AddColumn,
    RemoveColumn(usize),
    ColumnName(usize, String),
    ColumnType(usize, String),
    ColumnNullable(usize, bool),
    ColumnDefault(usize, String),
    ColumnAutoInc(usize, bool),
    ColumnIsPk(usize, bool),
    Engine(String),
    Charset(String),
    Collation(String),
    Submit,
    Done(Result<String, ApiClientError>),
}

pub struct TableCreate {
    name: String,
    columns: Vec<ColumnDraft>,
    engine: String,
    charset: String,
    collation: String,
    busy: bool,
    error: Option<String>,
    api_error: Option<ApiClientError>,
}

impl Component for TableCreate {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            name: String::new(),
            columns: vec![ColumnDraft::starter_pk()],
            engine: String::new(),
            charset: String::new(),
            collation: String::new(),
            busy: false,
            error: None,
            api_error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::TableName(s) => {
                self.name = s;
                true
            }
            Msg::AddColumn => {
                self.columns.push(ColumnDraft::default());
                true
            }
            Msg::RemoveColumn(i) => {
                if self.columns.len() > 1 {
                    self.columns.remove(i);
                }
                true
            }
            Msg::ColumnName(i, v) => {
                if let Some(c) = self.columns.get_mut(i) {
                    c.name = v;
                }
                true
            }
            Msg::ColumnType(i, v) => {
                if let Some(c) = self.columns.get_mut(i) {
                    c.data_type = v;
                }
                true
            }
            Msg::ColumnNullable(i, v) => {
                if let Some(c) = self.columns.get_mut(i) {
                    c.nullable = v;
                }
                true
            }
            Msg::ColumnDefault(i, v) => {
                if let Some(c) = self.columns.get_mut(i) {
                    c.default = v;
                }
                true
            }
            Msg::ColumnAutoInc(i, v) => {
                if let Some(c) = self.columns.get_mut(i) {
                    c.auto_increment = v;
                }
                true
            }
            Msg::ColumnIsPk(i, v) => {
                if let Some(c) = self.columns.get_mut(i) {
                    c.is_pk = v;
                }
                true
            }
            Msg::Engine(s) => {
                self.engine = s;
                true
            }
            Msg::Charset(s) => {
                self.charset = s;
                true
            }
            Msg::Collation(s) => {
                self.collation = s;
                true
            }
            Msg::Submit => {
                let req = match self.build_request() {
                    Ok(r) => r,
                    Err(msg) => {
                        self.error = Some(msg);
                        return true;
                    }
                };
                self.error = None;
                self.api_error = None;
                self.busy = true;
                let db = ctx.props().db.clone();
                let name = req.name.clone();
                ctx.link().send_future(async move {
                    Msg::Done(api::create_table(&db, &req).await.map(|_| name))
                });
                true
            }
            Msg::Done(Ok(name)) => {
                self.busy = false;
                ctx.props().on_created.emit(name);
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
        let add_col = ctx.link().callback(|_| Msg::AddColumn);
        let submit = ctx.link().callback(|_| Msg::Submit);
        let can_submit = !self.busy
            && !self.name.trim().is_empty()
            && self
                .columns
                .iter()
                .all(|c| !c.name.trim().is_empty() && !c.data_type.trim().is_empty());

        html! {
            <Modal
                title={AttrValue::from("Create table")}
                on_close={close_cb}
                width_class={AttrValue::from("max-w-4xl")}
            >
                <div class="space-y-5">
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                        <Labeled label="Table name" hint="Required.">
                            <TextInput
                                value={AttrValue::from(self.name.clone())}
                                placeholder="my_table"
                                oninput={ctx.link().callback(Msg::TableName)}
                            />
                        </Labeled>
                        <Labeled label="Database" hint="Target database (read-only).">
                            <TextInput value={AttrValue::from(ctx.props().db.clone())} />
                        </Labeled>
                    </div>

                    <div class="space-y-3">
                        <div class="flex items-center justify-between">
                            <h3 class="font-display font-semibold tracking-tight">{ "Columns" }</h3>
                            <Button variant={ButtonVariant::Secondary} onclick={add_col}>
                                { Html::from("+ Add column") }
                            </Button>
                        </div>
                        <div class="border border-border rounded-[12px] overflow-hidden">
                            <table class="w-full border-collapse text-sm">
                                <thead class="border-b border-border bg-background/60">
                                    <tr>
                                        <th class="text-left px-3 py-2 font-display text-[12px] font-semibold">{ "Name" }</th>
                                        <th class="text-left px-3 py-2 font-display text-[12px] font-semibold">{ "Type" }</th>
                                        <th class="text-center px-3 py-2 font-display text-[12px] font-semibold">{ "Null" }</th>
                                        <th class="text-left px-3 py-2 font-display text-[12px] font-semibold">{ "Default" }</th>
                                        <th class="text-center px-3 py-2 font-display text-[12px] font-semibold">{ "AI" }</th>
                                        <th class="text-center px-3 py-2 font-display text-[12px] font-semibold">{ "PK" }</th>
                                        <th class="px-2 py-2"></th>
                                    </tr>
                                </thead>
                                <tbody>
                                    { for self.columns.iter().enumerate().map(|(i, c)| self.view_column_row(ctx, i, c)) }
                                </tbody>
                            </table>
                        </div>
                    </div>

                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                        <Labeled label="Engine" hint="Optional. Default: server config.">
                            <TextInput
                                value={AttrValue::from(self.engine.clone())}
                                placeholder="InnoDB"
                                oninput={ctx.link().callback(Msg::Engine)}
                            />
                        </Labeled>
                        <Labeled label="Charset" hint="Optional.">
                            <TextInput
                                value={AttrValue::from(self.charset.clone())}
                                placeholder="utf8mb4"
                                oninput={ctx.link().callback(Msg::Charset)}
                            />
                        </Labeled>
                        <Labeled label="Collation" hint="Optional.">
                            <TextInput
                                value={AttrValue::from(self.collation.clone())}
                                placeholder="utf8mb4_0900_ai_ci"
                                oninput={ctx.link().callback(Msg::Collation)}
                            />
                        </Labeled>
                    </div>

                    if let Some(msg) = &self.error {
                        <div class="bg-error/5 border border-error/30 rounded-[12px] p-3 text-sm text-error">
                            { msg }
                        </div>
                    }
                    if let Some(e) = &self.api_error {
                        <ErrorBanner error={e.clone()} />
                    }

                    <div class="flex justify-end gap-2 pt-2">
                        <Button
                            variant={ButtonVariant::Secondary}
                            disabled={self.busy}
                            onclick={cancel_cb}
                        >
                            { Html::from("Cancel") }
                        </Button>
                        <Button
                            variant={ButtonVariant::Primary}
                            disabled={!can_submit}
                            onclick={submit}
                        >
                            { Html::from(if self.busy { "Creating…" } else { "Create table" }) }
                        </Button>
                    </div>
                </div>
            </Modal>
        }
    }
}

impl TableCreate {
    fn view_column_row(&self, ctx: &Context<Self>, i: usize, c: &ColumnDraft) -> Html {
        let can_remove = self.columns.len() > 1;
        let on_name = ctx.link().callback(move |v| Msg::ColumnName(i, v));
        let on_type = ctx.link().callback(move |v| Msg::ColumnType(i, v));
        let on_null = ctx
            .link()
            .callback(move |e: Event| Msg::ColumnNullable(i, checkbox_value(&e)));
        let on_default = ctx.link().callback(move |v| Msg::ColumnDefault(i, v));
        let on_ai = ctx
            .link()
            .callback(move |e: Event| Msg::ColumnAutoInc(i, checkbox_value(&e)));
        let on_pk = ctx
            .link()
            .callback(move |e: Event| Msg::ColumnIsPk(i, checkbox_value(&e)));
        let on_remove = ctx.link().callback(move |_| Msg::RemoveColumn(i));

        html! {
            <tr class="border-b border-border last:border-b-0 align-top">
                <td class="px-3 py-2 w-[20%]">
                    <TextInput
                        value={AttrValue::from(c.name.clone())}
                        placeholder="column"
                        oninput={on_name}
                    />
                </td>
                <td class="px-3 py-2 w-[24%]">
                    <TextInput
                        value={AttrValue::from(c.data_type.clone())}
                        placeholder="INT, VARCHAR(64), …"
                        oninput={on_type}
                    />
                </td>
                <td class="px-3 py-2 text-center">
                    <input type="checkbox" checked={c.nullable} onchange={on_null} />
                </td>
                <td class="px-3 py-2 w-[24%]">
                    <TextInput
                        value={AttrValue::from(c.default.clone())}
                        placeholder="NULL / 0 / 'x' / CURRENT_TIMESTAMP"
                        oninput={on_default}
                    />
                </td>
                <td class="px-3 py-2 text-center">
                    <input type="checkbox" checked={c.auto_increment} onchange={on_ai} />
                </td>
                <td class="px-3 py-2 text-center">
                    <input type="checkbox" checked={c.is_pk} onchange={on_pk} />
                </td>
                <td class="px-2 py-2 text-right">
                    <button
                        class={theme::BTN_GHOST}
                        type="button"
                        disabled={!can_remove}
                        onclick={on_remove}
                        title="Remove column"
                    >
                        { "×" }
                    </button>
                </td>
            </tr>
        }
    }

    fn build_request(&self) -> Result<CreateTableRequest, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Table name is required.".into());
        }
        if self.columns.is_empty() {
            return Err("At least one column is required.".into());
        }
        let mut columns = Vec::with_capacity(self.columns.len());
        let mut primary_key = Vec::new();
        for c in &self.columns {
            let col_name = c.name.trim();
            let data_type = c.data_type.trim();
            if col_name.is_empty() {
                return Err("Each column needs a name.".into());
            }
            if data_type.is_empty() {
                return Err(format!("Column `{col_name}` needs a data type."));
            }
            columns.push(ColumnDefinition {
                name: col_name.to_owned(),
                data_type: data_type.to_owned(),
                nullable: c.nullable,
                default: trim_opt(&c.default),
                auto_increment: c.auto_increment,
                comment: None,
            });
            if c.is_pk {
                primary_key.push(col_name.to_owned());
            }
        }
        Ok(CreateTableRequest {
            name: name.to_owned(),
            columns,
            primary_key,
            engine: trim_opt(&self.engine),
            charset: trim_opt(&self.charset),
            collation: trim_opt(&self.collation),
            comment: None,
            if_not_exists: false,
        })
    }
}

fn checkbox_value(e: &Event) -> bool {
    use wasm_bindgen::JsCast;
    e.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.checked())
        .unwrap_or(false)
}

fn trim_opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

#[derive(Properties, PartialEq)]
struct LabeledProps {
    label: &'static str,
    #[prop_or_default]
    hint: &'static str,
    children: Children,
}

#[function_component(Labeled)]
fn labeled(p: &LabeledProps) -> Html {
    html! {
        <label class="block space-y-1.5">
            <span class="block text-[13px] font-medium text-text">{ p.label }</span>
            { for p.children.iter() }
            if !p.hint.is_empty() {
                <span class="block text-xs text-text-secondary">{ p.hint }</span>
            }
        </label>
    }
}
