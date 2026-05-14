use mysqlview_types::{
    AlterTableOperation, AlterTableRequest, ColumnDefinition, ColumnInfo, TableStructure,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::button::{Button, ButtonVariant};
use crate::components::error_banner::ErrorBanner;
use crate::components::input::TextInput;
use crate::components::modal::Modal;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
    pub table: String,
    pub structure: TableStructure,
    pub on_close: Callback<()>,
    /// Fires after a successful ALTER. The argument is the (possibly new)
    /// table name so the caller can navigate when a RENAME TO is committed.
    pub on_changed: Callback<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    AddColumn,
    DropColumn,
    ModifyColumn,
    RenameColumn,
    RenameTable,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::AddColumn => "Add column",
            Self::DropColumn => "Drop column",
            Self::ModifyColumn => "Modify column",
            Self::RenameColumn => "Rename column",
            Self::RenameTable => "Rename table",
        }
    }

    fn all() -> [Self; 5] {
        [
            Self::AddColumn,
            Self::DropColumn,
            Self::ModifyColumn,
            Self::RenameColumn,
            Self::RenameTable,
        ]
    }
}

#[derive(Default, Clone)]
struct ColumnForm {
    name: String,
    data_type: String,
    nullable: bool,
    default: String,
    auto_increment: bool,
}

pub enum Msg {
    SetKind(Kind),
    AddName(String),
    AddType(String),
    AddNullable(bool),
    AddDefault(String),
    AddAutoInc(bool),
    AddAfter(String),
    DropName(String),
    ModifyTarget(String),
    ModifyType(String),
    ModifyNullable(bool),
    ModifyDefault(String),
    ModifyAutoInc(bool),
    RenameFrom(String),
    RenameTo(String),
    RenameTableTo(String),
    Submit,
    Done(Result<String, ApiClientError>),
}

pub struct TableAlter {
    kind: Kind,
    add: ColumnForm,
    add_after: String,
    drop_name: String,
    modify_target: String,
    modify: ColumnForm,
    rename_from: String,
    rename_to: String,
    rename_table_to: String,
    busy: bool,
    error: Option<String>,
    api_error: Option<ApiClientError>,
}

impl Component for TableAlter {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let first = ctx
            .props()
            .structure
            .columns
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_default();
        Self {
            kind: Kind::AddColumn,
            add: ColumnForm::default(),
            add_after: String::new(),
            drop_name: first.clone(),
            modify_target: first.clone(),
            modify: ColumnForm::default(),
            rename_from: first,
            rename_to: String::new(),
            rename_table_to: String::new(),
            busy: false,
            error: None,
            api_error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetKind(k) => {
                self.kind = k;
                self.error = None;
                self.api_error = None;
                if k == Kind::ModifyColumn {
                    self.preload_modify(ctx);
                }
                true
            }
            Msg::AddName(s) => {
                self.add.name = s;
                true
            }
            Msg::AddType(s) => {
                self.add.data_type = s;
                true
            }
            Msg::AddNullable(v) => {
                self.add.nullable = v;
                true
            }
            Msg::AddDefault(s) => {
                self.add.default = s;
                true
            }
            Msg::AddAutoInc(v) => {
                self.add.auto_increment = v;
                true
            }
            Msg::AddAfter(s) => {
                self.add_after = s;
                true
            }
            Msg::DropName(s) => {
                self.drop_name = s;
                true
            }
            Msg::ModifyTarget(s) => {
                self.modify_target = s;
                self.preload_modify(ctx);
                true
            }
            Msg::ModifyType(s) => {
                self.modify.data_type = s;
                true
            }
            Msg::ModifyNullable(v) => {
                self.modify.nullable = v;
                true
            }
            Msg::ModifyDefault(s) => {
                self.modify.default = s;
                true
            }
            Msg::ModifyAutoInc(v) => {
                self.modify.auto_increment = v;
                true
            }
            Msg::RenameFrom(s) => {
                self.rename_from = s;
                true
            }
            Msg::RenameTo(s) => {
                self.rename_to = s;
                true
            }
            Msg::RenameTableTo(s) => {
                self.rename_table_to = s;
                true
            }
            Msg::Submit => {
                let op = match self.build_operation() {
                    Ok(o) => o,
                    Err(msg) => {
                        self.error = Some(msg);
                        return true;
                    }
                };
                let new_name = if let AlterTableOperation::RenameTable { to } = &op {
                    to.clone()
                } else {
                    ctx.props().table.clone()
                };
                self.error = None;
                self.api_error = None;
                self.busy = true;
                let db = ctx.props().db.clone();
                let table = ctx.props().table.clone();
                let req = AlterTableRequest {
                    operations: vec![op],
                };
                ctx.link().send_future(async move {
                    Msg::Done(api::alter_table(&db, &table, &req).await.map(|_| new_name))
                });
                true
            }
            Msg::Done(Ok(name)) => {
                self.busy = false;
                ctx.props().on_changed.emit(name);
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
        let kind_change = ctx.link().callback(|e: Event| {
            let value = select_value(&e);
            Msg::SetKind(parse_kind(&value).unwrap_or(Kind::AddColumn))
        });

        html! {
            <Modal
                title={AttrValue::from("Alter table")}
                on_close={close_cb}
                width_class={AttrValue::from("max-w-3xl")}
            >
                <div class="space-y-5">
                    <Labeled label="Operation" hint="">
                        <select class={crate::theme::INPUT} onchange={kind_change}>
                            { for Kind::all().iter().map(|k| html! {
                                <option value={kind_value(*k)} selected={*k == self.kind}>
                                    { k.label() }
                                </option>
                            }) }
                        </select>
                    </Labeled>

                    { self.view_form(ctx) }

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
                            disabled={self.busy}
                            onclick={submit}
                        >
                            { Html::from(if self.busy { "Applying…" } else { "Apply change" }) }
                        </Button>
                    </div>
                </div>
            </Modal>
        }
    }
}

impl TableAlter {
    fn columns<'a>(&self, ctx: &'a Context<Self>) -> &'a [ColumnInfo] {
        &ctx.props().structure.columns
    }

    fn preload_modify(&mut self, ctx: &Context<Self>) {
        if let Some(col) = self
            .columns(ctx)
            .iter()
            .find(|c| c.name == self.modify_target)
        {
            self.modify = ColumnForm {
                name: col.name.clone(),
                data_type: col.data_type.clone(),
                nullable: col.nullable,
                default: col.default.clone().unwrap_or_default(),
                auto_increment: col
                    .extra
                    .as_deref()
                    .is_some_and(|e| e.eq_ignore_ascii_case("auto_increment")),
            };
        }
    }

    fn view_form(&self, ctx: &Context<Self>) -> Html {
        match self.kind {
            Kind::AddColumn => self.view_add_form(ctx),
            Kind::DropColumn => self.view_drop_form(ctx),
            Kind::ModifyColumn => self.view_modify_form(ctx),
            Kind::RenameColumn => self.view_rename_column_form(ctx),
            Kind::RenameTable => self.view_rename_table_form(ctx),
        }
    }

    fn view_add_form(&self, ctx: &Context<Self>) -> Html {
        let on_after = ctx
            .link()
            .callback(|e: Event| Msg::AddAfter(select_value(&e)));
        html! {
            <div class="space-y-4">
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <Labeled label="Name" hint="Required.">
                        <TextInput
                            value={AttrValue::from(self.add.name.clone())}
                            placeholder="new_column"
                            oninput={ctx.link().callback(Msg::AddName)}
                        />
                    </Labeled>
                    <Labeled label="Type" hint="Required.">
                        <TextInput
                            value={AttrValue::from(self.add.data_type.clone())}
                            placeholder="INT, VARCHAR(64), …"
                            oninput={ctx.link().callback(Msg::AddType)}
                        />
                    </Labeled>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <Labeled label="Default" hint="Optional. NULL / 0 / 'x' / CURRENT_TIMESTAMP">
                        <TextInput
                            value={AttrValue::from(self.add.default.clone())}
                            placeholder=""
                            oninput={ctx.link().callback(Msg::AddDefault)}
                        />
                    </Labeled>
                    <Labeled label="Insert after" hint="Optional. Default: append at end.">
                        <select class={crate::theme::INPUT} onchange={on_after}>
                            <option value="" selected={self.add_after.is_empty()}>{ "(end)" }</option>
                            { for self.columns(ctx).iter().map(|c| html! {
                                <option value={c.name.clone()} selected={c.name == self.add_after}>
                                    { &c.name }
                                </option>
                            }) }
                        </select>
                    </Labeled>
                </div>
                <div class="flex gap-4 text-sm">
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input
                            type="checkbox"
                            checked={self.add.nullable}
                            onchange={ctx.link().callback(|e: Event| Msg::AddNullable(checkbox_value(&e)))}
                        />
                        { "NULL" }
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input
                            type="checkbox"
                            checked={self.add.auto_increment}
                            onchange={ctx.link().callback(|e: Event| Msg::AddAutoInc(checkbox_value(&e)))}
                        />
                        { "AUTO_INCREMENT" }
                    </label>
                </div>
            </div>
        }
    }

    fn view_drop_form(&self, ctx: &Context<Self>) -> Html {
        let on_change = ctx
            .link()
            .callback(|e: Event| Msg::DropName(select_value(&e)));
        html! {
            <Labeled label="Column" hint="The column will be permanently removed.">
                <select class={crate::theme::INPUT} onchange={on_change}>
                    { for self.columns(ctx).iter().map(|c| html! {
                        <option value={c.name.clone()} selected={c.name == self.drop_name}>
                            { &c.name }
                        </option>
                    }) }
                </select>
            </Labeled>
        }
    }

    fn view_modify_form(&self, ctx: &Context<Self>) -> Html {
        let on_target = ctx
            .link()
            .callback(|e: Event| Msg::ModifyTarget(select_value(&e)));
        html! {
            <div class="space-y-4">
                <Labeled label="Column" hint="">
                    <select class={crate::theme::INPUT} onchange={on_target}>
                        { for self.columns(ctx).iter().map(|c| html! {
                            <option value={c.name.clone()} selected={c.name == self.modify_target}>
                                { &c.name }
                            </option>
                        }) }
                    </select>
                </Labeled>
                <Labeled label="New type" hint="Required.">
                    <TextInput
                        value={AttrValue::from(self.modify.data_type.clone())}
                        placeholder="INT, VARCHAR(64), …"
                        oninput={ctx.link().callback(Msg::ModifyType)}
                    />
                </Labeled>
                <Labeled label="Default" hint="Optional.">
                    <TextInput
                        value={AttrValue::from(self.modify.default.clone())}
                        placeholder=""
                        oninput={ctx.link().callback(Msg::ModifyDefault)}
                    />
                </Labeled>
                <div class="flex gap-4 text-sm">
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input
                            type="checkbox"
                            checked={self.modify.nullable}
                            onchange={ctx.link().callback(|e: Event| Msg::ModifyNullable(checkbox_value(&e)))}
                        />
                        { "NULL" }
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input
                            type="checkbox"
                            checked={self.modify.auto_increment}
                            onchange={ctx.link().callback(|e: Event| Msg::ModifyAutoInc(checkbox_value(&e)))}
                        />
                        { "AUTO_INCREMENT" }
                    </label>
                </div>
            </div>
        }
    }

    fn view_rename_column_form(&self, ctx: &Context<Self>) -> Html {
        let on_from = ctx
            .link()
            .callback(|e: Event| Msg::RenameFrom(select_value(&e)));
        html! {
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <Labeled label="From" hint="">
                    <select class={crate::theme::INPUT} onchange={on_from}>
                        { for self.columns(ctx).iter().map(|c| html! {
                            <option value={c.name.clone()} selected={c.name == self.rename_from}>
                                { &c.name }
                            </option>
                        }) }
                    </select>
                </Labeled>
                <Labeled label="To" hint="Required.">
                    <TextInput
                        value={AttrValue::from(self.rename_to.clone())}
                        placeholder="new_name"
                        oninput={ctx.link().callback(Msg::RenameTo)}
                    />
                </Labeled>
            </div>
        }
    }

    fn view_rename_table_form(&self, ctx: &Context<Self>) -> Html {
        html! {
            <Labeled label="New table name" hint="Required.">
                <TextInput
                    value={AttrValue::from(self.rename_table_to.clone())}
                    placeholder="new_table"
                    oninput={ctx.link().callback(Msg::RenameTableTo)}
                />
            </Labeled>
        }
    }

    fn build_operation(&self) -> Result<AlterTableOperation, String> {
        match self.kind {
            Kind::AddColumn => {
                if self.add.name.trim().is_empty() {
                    return Err("Column name is required.".into());
                }
                if self.add.data_type.trim().is_empty() {
                    return Err("Column type is required.".into());
                }
                Ok(AlterTableOperation::AddColumn {
                    column: ColumnDefinition {
                        name: self.add.name.trim().to_owned(),
                        data_type: self.add.data_type.trim().to_owned(),
                        nullable: self.add.nullable,
                        default: trim_opt(&self.add.default),
                        auto_increment: self.add.auto_increment,
                        comment: None,
                    },
                    after: trim_opt(&self.add_after),
                })
            }
            Kind::DropColumn => {
                if self.drop_name.is_empty() {
                    return Err("Pick a column to drop.".into());
                }
                Ok(AlterTableOperation::DropColumn {
                    name: self.drop_name.clone(),
                })
            }
            Kind::ModifyColumn => {
                if self.modify_target.is_empty() {
                    return Err("Pick a column to modify.".into());
                }
                if self.modify.data_type.trim().is_empty() {
                    return Err("New type is required.".into());
                }
                Ok(AlterTableOperation::ModifyColumn {
                    column: ColumnDefinition {
                        name: self.modify_target.clone(),
                        data_type: self.modify.data_type.trim().to_owned(),
                        nullable: self.modify.nullable,
                        default: trim_opt(&self.modify.default),
                        auto_increment: self.modify.auto_increment,
                        comment: None,
                    },
                })
            }
            Kind::RenameColumn => {
                if self.rename_from.is_empty() {
                    return Err("Pick a column to rename.".into());
                }
                let to = self.rename_to.trim();
                if to.is_empty() {
                    return Err("New column name is required.".into());
                }
                Ok(AlterTableOperation::RenameColumn {
                    from: self.rename_from.clone(),
                    to: to.to_owned(),
                })
            }
            Kind::RenameTable => {
                let to = self.rename_table_to.trim();
                if to.is_empty() {
                    return Err("New table name is required.".into());
                }
                Ok(AlterTableOperation::RenameTable { to: to.to_owned() })
            }
        }
    }
}

fn kind_value(k: Kind) -> &'static str {
    match k {
        Kind::AddColumn => "add",
        Kind::DropColumn => "drop",
        Kind::ModifyColumn => "modify",
        Kind::RenameColumn => "rename_col",
        Kind::RenameTable => "rename_table",
    }
}

fn parse_kind(s: &str) -> Option<Kind> {
    Some(match s {
        "add" => Kind::AddColumn,
        "drop" => Kind::DropColumn,
        "modify" => Kind::ModifyColumn,
        "rename_col" => Kind::RenameColumn,
        "rename_table" => Kind::RenameTable,
        _ => return None,
    })
}

fn select_value(e: &Event) -> String {
    e.target()
        .and_then(|t| t.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default()
}

fn checkbox_value(e: &Event) -> bool {
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
