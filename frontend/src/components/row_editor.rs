use std::collections::BTreeMap;

use mysqlview_types::{CellValue, ColumnInfo, RowValues};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::components::button::{Button, ButtonVariant};
use crate::theme;

/// How a column should be rendered in the editor and how its raw string is
/// converted between the MySQL on-wire format and the HTML form representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    /// Single-line text input (default).
    Text,
    /// `<textarea>` for TEXT / JSON-style columns.
    LongText,
    /// Read-only display for BLOB / BIT / GEOMETRY values.
    Bytes,
    /// `<input type="date">` — `YYYY-MM-DD`.
    Date,
    /// `<input type="datetime-local" step="1">` — `YYYY-MM-DDTHH:MM:SS`.
    /// MySQL DATETIME / TIMESTAMP values use a space between date and time;
    /// we convert at the form boundary.
    DateTime,
    /// `<input type="time" step="1">` — `HH:MM:SS`.
    Time,
}

/// Per-column draft state. We keep the user-visible text and a NULL toggle
/// separately so we can render a stable form even when MySQL types lose their
/// natural string representation (e.g. binary data).
#[derive(Debug, Clone, PartialEq)]
struct FieldDraft {
    /// Always stored in the *form* representation (i.e. with `T` separator for
    /// DATETIME). `collect_values` converts back to MySQL format on submit.
    text: String,
    is_null: bool,
    /// `true` for BLOB / BIT / GEOMETRY — value is shown as readonly metadata.
    readonly_bytes: bool,
    /// Optional preserved bytes for BLOB columns coming from an existing row.
    original_bytes_b64: Option<String>,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub columns: Vec<ColumnInfo>,
    /// Initial values keyed by column name. Missing columns are treated as NULL.
    /// When empty, the editor renders an "insert" form (NULLable columns default
    /// to NULL, others to empty string).
    #[prop_or_default]
    pub initial: RowValues,
    /// `true` for the "Add row" flow. AUTO_INCREMENT columns become optional.
    #[prop_or(false)]
    pub is_insert: bool,
    pub on_submit: Callback<RowValues>,
    pub on_cancel: Callback<()>,
    #[prop_or(false)]
    pub busy: bool,
}

pub enum Msg {
    SetText(String, String),
    ToggleNull(String, bool),
    Submit,
    Cancel,
}

pub struct RowEditor {
    fields: BTreeMap<String, FieldDraft>,
}

impl Component for RowEditor {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let p = ctx.props();
        let fields = build_initial_fields(&p.columns, &p.initial, p.is_insert);
        Self { fields }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetText(col, text) => {
                if let Some(f) = self.fields.get_mut(&col) {
                    f.text = text;
                    f.is_null = false;
                }
                true
            }
            Msg::ToggleNull(col, value) => {
                if let Some(f) = self.fields.get_mut(&col) {
                    f.is_null = value;
                }
                true
            }
            Msg::Submit => {
                let values = self.collect_values(ctx);
                ctx.props().on_submit.emit(values);
                false
            }
            Msg::Cancel => {
                ctx.props().on_cancel.emit(());
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let on_submit = ctx.link().callback(|_| Msg::Submit);
        let on_cancel = ctx.link().callback(|_| Msg::Cancel);

        html! {
            <div class="space-y-4">
                <div class="space-y-3 max-h-[60vh] overflow-y-auto pr-1">
                    { for p.columns.iter().map(|c| self.view_field(ctx, c)) }
                </div>
                <div class="flex justify-end gap-2 pt-2 border-t border-border">
                    <Button variant={ButtonVariant::Secondary} disabled={p.busy} onclick={on_cancel}>
                        { Html::from("Cancel") }
                    </Button>
                    <Button variant={ButtonVariant::Primary} disabled={p.busy} onclick={on_submit}>
                        { Html::from(if p.busy { "Saving…" } else { "Save" }) }
                    </Button>
                </div>
            </div>
        }
    }
}

impl RowEditor {
    fn collect_values(&self, ctx: &Context<Self>) -> RowValues {
        let p = ctx.props();
        let mut out: RowValues = BTreeMap::new();
        for col in &p.columns {
            let Some(draft) = self.fields.get(&col.name) else {
                continue;
            };
            if draft.is_null {
                if col.nullable {
                    out.insert(col.name.clone(), CellValue::Null);
                }
                continue;
            }
            if draft.readonly_bytes {
                if let Some(b64) = &draft.original_bytes_b64 {
                    out.insert(
                        col.name.clone(),
                        CellValue::Bytes {
                            base64: b64.clone(),
                        },
                    );
                }
                continue;
            }
            let kind = classify_column(col);
            // Auto-increment / DEFAULT-handled columns: skip if blank on insert.
            if p.is_insert && is_auto_increment(col) && draft.text.trim().is_empty() {
                continue;
            }
            out.insert(
                col.name.clone(),
                CellValue::String(input_to_mysql(kind, &draft.text)),
            );
        }
        out
    }

    fn view_field(&self, ctx: &Context<Self>, col: &ColumnInfo) -> Html {
        let draft = self
            .fields
            .get(&col.name)
            .cloned()
            .unwrap_or_else(|| empty_draft(col, false));
        let kind = classify_column(col);
        let col_name = col.name.clone();
        let null_col = col.name.clone();
        let on_input_link = ctx.link().clone();
        let on_input = Callback::from(move |e: InputEvent| {
            let value = target_value(&e);
            on_input_link.send_message(Msg::SetText(col_name.clone(), value));
        });
        let on_null = ctx
            .link()
            .callback(move |e: Event| Msg::ToggleNull(null_col.clone(), checkbox_value(&e)));

        let header = html! {
            <div class="flex items-center justify-between gap-3">
                <div class="text-sm">
                    <span class="font-medium font-mono">{ &col.name }</span>
                    <span class="text-xs text-text-secondary ml-2 font-mono">{ &col.data_type }</span>
                    if !col.nullable {
                        <span class="text-xs text-warning ml-2">{ "NOT NULL" }</span>
                    }
                    if is_auto_increment(col) {
                        <span class="text-xs text-text-secondary ml-2">{ "(auto)" }</span>
                    }
                </div>
                if col.nullable {
                    <label class="text-xs text-text-secondary flex items-center gap-1 cursor-pointer">
                        <input
                            type="checkbox"
                            class="accent-primary"
                            checked={draft.is_null}
                            onchange={on_null}
                        />
                        { "NULL" }
                    </label>
                }
            </div>
        };

        let body = match kind {
            FieldKind::Bytes => html! {
                <div class="px-3 py-2 text-xs text-text-secondary bg-background/60 border border-border rounded-[6px] font-mono">
                    { "Binary data — editing not supported in this view." }
                </div>
            },
            FieldKind::LongText => html! {
                <textarea
                    class={format!("{} font-mono leading-relaxed", theme::INPUT)}
                    rows="3"
                    placeholder={placeholder_for(col, ctx.props().is_insert)}
                    disabled={draft.is_null}
                    value={draft.text.clone()}
                    oninput={on_input}
                />
            },
            FieldKind::Date | FieldKind::DateTime | FieldKind::Time => {
                self.view_temporal(ctx, col, kind, &draft, on_input)
            }
            FieldKind::Text => html! {
                <input
                    class={theme::INPUT}
                    type="text"
                    placeholder={placeholder_for(col, ctx.props().is_insert)}
                    disabled={draft.is_null}
                    value={draft.text.clone()}
                    oninput={on_input}
                />
            },
        };

        html! {
            <div class="space-y-1.5">
                { header }
                { body }
            </div>
        }
    }

    fn view_temporal(
        &self,
        ctx: &Context<Self>,
        col: &ColumnInfo,
        kind: FieldKind,
        draft: &FieldDraft,
        on_input: Callback<InputEvent>,
    ) -> Html {
        let input_type = match kind {
            FieldKind::Date => "date",
            FieldKind::DateTime => "datetime-local",
            FieldKind::Time => "time",
            _ => "text",
        };
        // step="1" enables seconds precision; "any" allows sub-second too.
        let step = match kind {
            FieldKind::Date => None,
            FieldKind::Time | FieldKind::DateTime => Some(AttrValue::from("1")),
            _ => None,
        };
        let col_name = col.name.clone();
        let on_now = ctx.link().callback(move |_e: MouseEvent| {
            Msg::SetText(col_name.clone(), current_local(kind).unwrap_or_default())
        });
        let now_label = match kind {
            FieldKind::Date => "Today",
            _ => "Now",
        };

        html! {
            <div class="flex gap-2 items-stretch">
                <input
                    class={format!("{} flex-1", theme::INPUT)}
                    type={input_type}
                    step={step.unwrap_or_default()}
                    placeholder={placeholder_for(col, ctx.props().is_insert)}
                    disabled={draft.is_null}
                    value={draft.text.clone()}
                    oninput={on_input}
                />
                <button
                    type="button"
                    class={theme::BTN_QUICK}
                    disabled={draft.is_null}
                    onclick={on_now}
                    title={AttrValue::from("Insert current local time")}
                >
                    { now_label }
                </button>
            </div>
        }
    }
}

fn build_initial_fields(
    columns: &[ColumnInfo],
    initial: &RowValues,
    is_insert: bool,
) -> BTreeMap<String, FieldDraft> {
    let mut out: BTreeMap<String, FieldDraft> = BTreeMap::new();
    for col in columns {
        let current = initial.get(&col.name);
        let mut draft = empty_draft(col, is_insert);
        if let Some(value) = current {
            apply_value(&mut draft, classify_column(col), value);
        }
        out.insert(col.name.clone(), draft);
    }
    out
}

fn empty_draft(col: &ColumnInfo, is_insert: bool) -> FieldDraft {
    let readonly_bytes = matches!(classify_column(col), FieldKind::Bytes);
    let is_null = if is_insert {
        col.nullable && !readonly_bytes && !is_auto_increment(col)
    } else {
        col.nullable
    };
    FieldDraft {
        text: String::new(),
        is_null,
        readonly_bytes,
        original_bytes_b64: None,
    }
}

fn apply_value(draft: &mut FieldDraft, kind: FieldKind, value: &CellValue) {
    match value {
        CellValue::Null => {
            draft.is_null = true;
            draft.text.clear();
        }
        CellValue::Bool(b) => {
            draft.is_null = false;
            draft.text = if *b { "1".into() } else { "0".into() };
        }
        CellValue::Int(n) => {
            draft.is_null = false;
            draft.text = n.to_string();
        }
        CellValue::Float(f) => {
            draft.is_null = false;
            draft.text = f.to_string();
        }
        CellValue::String(s) => {
            draft.is_null = false;
            draft.text = mysql_to_input(kind, s);
        }
        CellValue::Bytes { base64 } => {
            draft.is_null = false;
            draft.readonly_bytes = true;
            draft.original_bytes_b64 = Some(base64.clone());
        }
        CellValue::Json(v) => {
            draft.is_null = false;
            draft.text = v.to_string();
        }
    }
}

fn classify_column(col: &ColumnInfo) -> FieldKind {
    let lower = col.data_type.to_ascii_lowercase();
    let base = lower
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .unwrap_or(&lower);
    match base {
        "blob" | "tinyblob" | "mediumblob" | "longblob" | "binary" | "varbinary" | "bit"
        | "geometry" | "point" | "linestring" | "polygon" => FieldKind::Bytes,
        "date" => FieldKind::Date,
        "datetime" | "timestamp" => FieldKind::DateTime,
        "time" => FieldKind::Time,
        "json" | "text" | "tinytext" | "mediumtext" | "longtext" => FieldKind::LongText,
        "char" | "varchar" | "enum" | "set" => FieldKind::Text,
        _ => FieldKind::Text,
    }
}

fn is_auto_increment(col: &ColumnInfo) -> bool {
    col.extra
        .as_deref()
        .map(|s| s.to_ascii_lowercase().contains("auto_increment"))
        .unwrap_or(false)
}

fn placeholder_for(col: &ColumnInfo, is_insert: bool) -> AttrValue {
    if is_insert && is_auto_increment(col) {
        return AttrValue::from("(auto)");
    }
    AttrValue::from(col.data_type.clone())
}

/// MySQL on-wire string → HTML form representation.
///
/// DATETIME / TIMESTAMP values come in as `YYYY-MM-DD HH:MM:SS[.fff…]`. The
/// `datetime-local` input requires a `T` separator. Other kinds pass through.
fn mysql_to_input(kind: FieldKind, value: &str) -> String {
    match kind {
        FieldKind::DateTime => value.replacen(' ', "T", 1),
        _ => value.to_owned(),
    }
}

/// HTML form representation → MySQL on-wire string.
fn input_to_mysql(kind: FieldKind, value: &str) -> String {
    match kind {
        FieldKind::DateTime => value.replacen('T', " ", 1),
        _ => value.to_owned(),
    }
}

/// Returns the current local time formatted for the given input kind. None for
/// kinds that have no natural "now" representation.
fn current_local(kind: FieldKind) -> Option<String> {
    let d = js_sys::Date::new_0();
    let y = d.get_full_year();
    let mo = d.get_month() + 1;
    let dy = d.get_date();
    let h = d.get_hours();
    let mi = d.get_minutes();
    let se = d.get_seconds();
    Some(match kind {
        FieldKind::Date => format!("{y:04}-{mo:02}-{dy:02}"),
        FieldKind::Time => format!("{h:02}:{mi:02}:{se:02}"),
        FieldKind::DateTime => format!("{y:04}-{mo:02}-{dy:02}T{h:02}:{mi:02}:{se:02}"),
        _ => return None,
    })
}

fn target_value(e: &InputEvent) -> String {
    if let Some(t) = e
        .target()
        .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    {
        return t.value();
    }
    e.target()
        .and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|t| t.value())
        .unwrap_or_default()
}

fn checkbox_value(e: &Event) -> bool {
    e.target()
        .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
        .map(|t| t.checked())
        .unwrap_or(false)
}
