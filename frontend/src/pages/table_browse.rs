use std::collections::BTreeMap;

use mysqlview_types::{
    BrowseRequest, BrowseResponse, CellValue, DeleteRowRequest, EditAffectedResponse, IndexInfo,
    InsertRowRequest, InsertRowResponse, RowValues, SortOrder, TableStructure, UpdateRowRequest,
};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::button::{Button, ButtonVariant};
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::csv_import::CsvImport;
use crate::components::data_grid::DataGrid;
use crate::components::error_banner::ErrorBanner;
use crate::components::modal::Modal;
use crate::components::pagination::Pagination;
use crate::components::row_editor::RowEditor;
use crate::components::skeleton::Skeleton;
use crate::router::Route;
use crate::state::LoadingState;
use crate::theme;
use crate::util::download::download_text;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub db: String,
    pub table: String,
}

#[derive(Clone, PartialEq)]
enum ActiveModal {
    None,
    Add,
    Edit(usize),
    Delete(usize),
    Import,
}

pub enum Msg {
    FetchAll,
    BrowseLoaded(Result<BrowseResponse, ApiClientError>),
    StructureLoaded(Result<TableStructure, ApiClientError>),
    ChangePage(u64),
    Sort(String),
    OpenAdd,
    OpenEdit(usize),
    OpenDelete(usize),
    OpenImport,
    CloseModal,
    SubmitInsert(RowValues),
    SubmitUpdate(RowValues),
    ConfirmDelete,
    InsertDone(Result<InsertRowResponse, ApiClientError>),
    UpdateDone(Result<EditAffectedResponse, ApiClientError>),
    DeleteDone(Result<EditAffectedResponse, ApiClientError>),
    ExportCsv,
    ExportSql,
    ExportDone(Result<(String, String, &'static str), ApiClientError>),
}

pub struct TableBrowsePage {
    state: LoadingState<BrowseResponse>,
    structure: LoadingState<TableStructure>,
    request: BrowseRequest,
    modal: ActiveModal,
    mutation_busy: bool,
    mutation_error: Option<ApiClientError>,
}

impl Component for TableBrowsePage {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::FetchAll);
        Self {
            state: LoadingState::Loading,
            structure: LoadingState::Loading,
            request: BrowseRequest::default(),
            modal: ActiveModal::None,
            mutation_busy: false,
            mutation_error: None,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old: &Self::Properties) -> bool {
        self.request = BrowseRequest::default();
        self.modal = ActiveModal::None;
        self.mutation_error = None;
        self.mutation_busy = false;
        ctx.link().send_message(Msg::FetchAll);
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FetchAll => {
                self.state = LoadingState::Loading;
                self.structure = LoadingState::Loading;
                let p = ctx.props();
                let db = p.db.clone();
                let table = p.table.clone();
                let req = self.request.clone();
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = api::browse_rows(&db, &table, &req).await;
                    link.send_message(Msg::BrowseLoaded(result));
                });
                let db = p.db.clone();
                let table = p.table.clone();
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = api::describe_table(&db, &table).await;
                    link.send_message(Msg::StructureLoaded(result));
                });
                true
            }
            Msg::BrowseLoaded(Ok(r)) => {
                self.state = LoadingState::Ready(r);
                true
            }
            Msg::BrowseLoaded(Err(e)) => {
                self.state = LoadingState::Failed(e);
                true
            }
            Msg::StructureLoaded(Ok(s)) => {
                self.structure = LoadingState::Ready(s);
                true
            }
            Msg::StructureLoaded(Err(e)) => {
                self.structure = LoadingState::Failed(e);
                true
            }
            Msg::ChangePage(offset) => {
                self.request.offset = offset;
                ctx.link().send_message(Msg::FetchAll);
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
                ctx.link().send_message(Msg::FetchAll);
                false
            }
            Msg::OpenAdd => {
                self.modal = ActiveModal::Add;
                self.mutation_error = None;
                true
            }
            Msg::OpenEdit(idx) => {
                self.modal = ActiveModal::Edit(idx);
                self.mutation_error = None;
                true
            }
            Msg::OpenDelete(idx) => {
                self.modal = ActiveModal::Delete(idx);
                self.mutation_error = None;
                true
            }
            Msg::CloseModal => {
                self.modal = ActiveModal::None;
                self.mutation_busy = false;
                true
            }
            Msg::SubmitInsert(values) => {
                self.mutation_busy = true;
                let p = ctx.props();
                let db = p.db.clone();
                let table = p.table.clone();
                let req = InsertRowRequest { values };
                ctx.link().send_future(async move {
                    Msg::InsertDone(api::insert_row(&db, &table, &req).await)
                });
                true
            }
            Msg::SubmitUpdate(set) => {
                let Some(key) = self.current_edit_key() else {
                    self.mutation_busy = false;
                    return true;
                };
                self.mutation_busy = true;
                let p = ctx.props();
                let db = p.db.clone();
                let table = p.table.clone();
                let req = UpdateRowRequest { key, set };
                ctx.link().send_future(async move {
                    Msg::UpdateDone(api::update_row(&db, &table, &req).await)
                });
                true
            }
            Msg::ConfirmDelete => {
                let Some(key) = self.current_delete_key() else {
                    self.mutation_busy = false;
                    return true;
                };
                self.mutation_busy = true;
                let p = ctx.props();
                let db = p.db.clone();
                let table = p.table.clone();
                let req = DeleteRowRequest { key };
                ctx.link().send_future(async move {
                    Msg::DeleteDone(api::delete_row(&db, &table, &req).await)
                });
                true
            }
            Msg::InsertDone(Ok(_)) | Msg::UpdateDone(Ok(_)) | Msg::DeleteDone(Ok(_)) => {
                self.modal = ActiveModal::None;
                self.mutation_busy = false;
                self.mutation_error = None;
                ctx.link().send_message(Msg::FetchAll);
                true
            }
            Msg::InsertDone(Err(e)) | Msg::UpdateDone(Err(e)) | Msg::DeleteDone(Err(e)) => {
                self.mutation_busy = false;
                self.mutation_error = Some(e);
                true
            }
            Msg::OpenImport => {
                self.modal = ActiveModal::Import;
                self.mutation_error = None;
                true
            }
            Msg::ExportCsv => {
                let db = ctx.props().db.clone();
                let table = ctx.props().table.clone();
                ctx.link().send_future(async move {
                    Msg::ExportDone(
                        api::export_table_csv(&db, &table)
                            .await
                            .map(|(name, body)| (name, body, "text/csv;charset=utf-8")),
                    )
                });
                false
            }
            Msg::ExportSql => {
                let db = ctx.props().db.clone();
                let table = ctx.props().table.clone();
                ctx.link().send_future(async move {
                    Msg::ExportDone(
                        api::export_table_sql(&db, &table)
                            .await
                            .map(|(name, body)| (name, body, "application/sql")),
                    )
                });
                false
            }
            Msg::ExportDone(Ok((filename, body, mime))) => {
                let p = ctx.props();
                let fallback = format!("{}__{}.txt", p.db, p.table);
                let name = if filename.is_empty() {
                    fallback.as_str()
                } else {
                    filename.as_str()
                };
                // download_text failing means the browser is missing DOM APIs
                // we expect to be present; nothing actionable here, so swallow.
                let _ = download_text(name, mime, &body);
                false
            }
            Msg::ExportDone(Err(e)) => {
                self.mutation_error = Some(e);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let editable_columns = self.editable_key_columns();
        let editable = editable_columns.is_some();
        let on_add = ctx.link().callback(|_| Msg::OpenAdd);
        let on_export_csv = ctx.link().callback(|_| Msg::ExportCsv);
        let on_export_sql = ctx.link().callback(|_| Msg::ExportSql);
        let on_import = ctx.link().callback(|_| Msg::OpenImport);

        html! {
            <div class="space-y-6">
                <div class="space-y-1">
                    <div class={theme::OVERLINE}>
                        <Link<Route> to={Route::Database { db: p.db.clone() }} classes="hover:text-text">
                            { &p.db }
                        </Link<Route>>
                        { " · browse" }
                    </div>
                    <div class="flex items-center justify-between gap-4 flex-wrap">
                        <h1 class={theme::SECTION_HEADING}>{ &p.table }</h1>
                        <div class="flex items-center gap-2 flex-wrap">
                            <Link<Route>
                                to={Route::Structure { db: p.db.clone(), table: p.table.clone() }}
                                classes={theme::BTN_GHOST}
                            >
                                { "Structure" }
                            </Link<Route>>
                            <Button variant={ButtonVariant::Secondary} onclick={on_export_csv}>
                                { Html::from("Export CSV") }
                            </Button>
                            <Button variant={ButtonVariant::Secondary} onclick={on_export_sql}>
                                { Html::from("Export SQL") }
                            </Button>
                            <Button
                                variant={ButtonVariant::Secondary}
                                disabled={!editable}
                                onclick={on_import}
                            >
                                { Html::from("Import CSV") }
                            </Button>
                            <Button
                                variant={ButtonVariant::Primary}
                                disabled={!editable}
                                onclick={on_add}
                            >
                                { Html::from("+ Add row") }
                            </Button>
                        </div>
                    </div>
                </div>
                if !editable && !matches!(self.structure, LoadingState::Loading | LoadingState::Idle) {
                    <div class="bg-warning/5 border border-warning/30 rounded-[12px] p-4 text-sm text-text">
                        { "This table has no primary key or NOT NULL UNIQUE index. Editing is disabled." }
                    </div>
                }
                if let Some(e) = &self.mutation_error {
                    <ErrorBanner error={e.clone()} />
                }
                { self.view_body(ctx, editable) }
                { self.view_modal(ctx) }
            </div>
        }
    }
}

impl TableBrowsePage {
    fn view_body(&self, ctx: &Context<Self>, editable: bool) -> Html {
        match &self.state {
            LoadingState::Idle | LoadingState::Loading => html! { <Skeleton rows={8} /> },
            LoadingState::Failed(e) => html! { <ErrorBanner error={e.clone()} /> },
            LoadingState::Ready(resp) => {
                let on_sort = ctx.link().callback(Msg::Sort);
                let on_change = ctx.link().callback(Msg::ChangePage);
                let on_edit_row = editable.then(|| ctx.link().callback(Msg::OpenEdit));
                let on_delete_row = editable.then(|| ctx.link().callback(Msg::OpenDelete));
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
                            on_edit_row={on_edit_row}
                            on_delete_row={on_delete_row}
                        />
                        <div class="text-xs text-text-secondary text-right">
                            { format!("loaded in {} ms", resp.duration_ms) }
                        </div>
                    </div>
                }
            }
        }
    }

    fn view_modal(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::CloseModal);
        // Import doesn't need the structure/state to be Ready — handle it before
        // the early returns below.
        if matches!(self.modal, ActiveModal::Import) {
            let p = ctx.props();
            let on_done = ctx.link().callback(|_| Msg::FetchAll);
            return html! {
                <CsvImport
                    db={p.db.clone()}
                    table={p.table.clone()}
                    on_close={on_close}
                    on_done={on_done}
                />
            };
        }

        let structure = match &self.structure {
            LoadingState::Ready(s) => s.clone(),
            _ => return Html::default(),
        };
        let response = match &self.state {
            LoadingState::Ready(r) => r,
            _ => return Html::default(),
        };

        match &self.modal {
            ActiveModal::None | ActiveModal::Import => Html::default(),
            ActiveModal::Add => {
                let on_submit = ctx.link().callback(Msg::SubmitInsert);
                let on_cancel = on_close.clone();
                html! {
                    <Modal title="Add row" on_close={on_close}>
                        <RowEditor
                            columns={structure.columns.clone()}
                            initial={RowValues::new()}
                            is_insert=true
                            on_submit={on_submit}
                            on_cancel={on_cancel}
                            busy={self.mutation_busy}
                        />
                    </Modal>
                }
            }
            ActiveModal::Edit(idx) => {
                let Some(initial) = row_to_values(response, *idx) else {
                    return Html::default();
                };
                let on_submit = ctx.link().callback(Msg::SubmitUpdate);
                let on_cancel = on_close.clone();
                html! {
                    <Modal title="Edit row" on_close={on_close}>
                        <RowEditor
                            columns={structure.columns.clone()}
                            initial={initial}
                            is_insert=false
                            on_submit={on_submit}
                            on_cancel={on_cancel}
                            busy={self.mutation_busy}
                        />
                    </Modal>
                }
            }
            ActiveModal::Delete(idx) => {
                let body = describe_delete(response, *idx, self.editable_key_columns().as_deref());
                let on_cancel = on_close.clone();
                let on_confirm = ctx.link().callback(|_| Msg::ConfirmDelete);
                html! {
                    <ConfirmDialog
                        title="Delete row"
                        body={AttrValue::from(body)}
                        confirm_label={AttrValue::from("Delete")}
                        on_confirm={on_confirm}
                        on_cancel={on_cancel}
                        busy={self.mutation_busy}
                    />
                }
            }
        }
    }

    fn editable_key_columns(&self) -> Option<Vec<String>> {
        match &self.structure {
            LoadingState::Ready(s) => editable_key(s),
            _ => None,
        }
    }

    fn current_edit_key(&self) -> Option<RowValues> {
        let ActiveModal::Edit(idx) = self.modal else {
            return None;
        };
        let response = match &self.state {
            LoadingState::Ready(r) => r,
            _ => return None,
        };
        let key_cols = self.editable_key_columns()?;
        build_key_from_row(response, idx, &key_cols)
    }

    fn current_delete_key(&self) -> Option<RowValues> {
        let ActiveModal::Delete(idx) = self.modal else {
            return None;
        };
        let response = match &self.state {
            LoadingState::Ready(r) => r,
            _ => return None,
        };
        let key_cols = self.editable_key_columns()?;
        build_key_from_row(response, idx, &key_cols)
    }
}

/// Picks the first usable identifying key from a table structure: the primary
/// key, or otherwise a UNIQUE index whose columns are all NOT NULL.
pub fn editable_key(structure: &TableStructure) -> Option<Vec<String>> {
    if let Some(pk) = structure.indexes.iter().find(|i| i.primary) {
        return Some(pk.columns.clone());
    }
    let nullable: BTreeMap<&str, bool> = structure
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.nullable))
        .collect();
    structure
        .indexes
        .iter()
        .find(|i| is_not_null_unique(i, &nullable))
        .map(|i| i.columns.clone())
}

fn is_not_null_unique(index: &IndexInfo, nullable: &BTreeMap<&str, bool>) -> bool {
    index.unique
        && !index.columns.is_empty()
        && index
            .columns
            .iter()
            .all(|c| nullable.get(c.as_str()).copied() == Some(false))
}

fn row_to_values(resp: &BrowseResponse, idx: usize) -> Option<RowValues> {
    let row = resp.rows.get(idx)?;
    let mut out: RowValues = BTreeMap::new();
    for (col, value) in resp.columns.iter().zip(row.iter()) {
        out.insert(col.clone(), value.clone());
    }
    Some(out)
}

fn build_key_from_row(resp: &BrowseResponse, idx: usize, key_cols: &[String]) -> Option<RowValues> {
    let row = resp.rows.get(idx)?;
    let mut out: RowValues = BTreeMap::new();
    for key in key_cols {
        let pos = resp.columns.iter().position(|c| c == key)?;
        out.insert(key.clone(), row.get(pos).cloned()?);
    }
    Some(out)
}

fn describe_delete(resp: &BrowseResponse, idx: usize, key_cols: Option<&[String]>) -> String {
    let Some(cols) = key_cols else {
        return "Are you sure you want to delete this row?".into();
    };
    let mut parts = Vec::new();
    if let Some(row) = resp.rows.get(idx) {
        for key in cols {
            if let Some(pos) = resp.columns.iter().position(|c| c == key) {
                let value = row.get(pos).map(cell_display).unwrap_or_default();
                parts.push(format!("{key} = {value}"));
            }
        }
    }
    if parts.is_empty() {
        "Are you sure you want to delete this row?".into()
    } else {
        format!(
            "Delete the row where {}? This cannot be undone.",
            parts.join(" AND ")
        )
    }
}

fn cell_display(c: &CellValue) -> String {
    match c {
        CellValue::Null => "NULL".into(),
        CellValue::Bool(b) => b.to_string(),
        CellValue::Int(n) => n.to_string(),
        CellValue::Float(f) => f.to_string(),
        CellValue::String(s) => format!("\"{s}\""),
        CellValue::Bytes { base64 } => format!("0x{} bytes", base64.len()),
        CellValue::Json(v) => v.to_string(),
    }
}
