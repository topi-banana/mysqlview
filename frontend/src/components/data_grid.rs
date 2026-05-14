use mysqlview_types::CellValue;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CellValue>>,
    #[prop_or_default]
    pub on_sort: Callback<String>,
    #[prop_or_default]
    pub sort_column: Option<String>,
}

pub struct DataGrid;

impl Component for DataGrid {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        if p.columns.is_empty() {
            return html! {
                <div class="bg-surface border border-border rounded-[12px] p-8 text-center text-text-secondary">
                    { "(query returned no columns)" }
                </div>
            };
        }
        html! {
            <div class="bg-surface border border-border rounded-[12px] overflow-hidden">
                <div class="overflow-x-auto max-h-[600px]">
                    <table class="w-full border-collapse text-sm">
                        <thead class="sticky top-0 bg-surface border-b border-border z-10">
                            <tr>
                                { for p.columns.iter().enumerate().map(|(idx, col)| self.view_th(ctx, idx, col)) }
                            </tr>
                        </thead>
                        <tbody>
                            { for p.rows.iter().map(|row| self.view_row(row, p.columns.len())) }
                            { if p.rows.is_empty() { self.view_empty(p.columns.len()) } else { Html::default() } }
                        </tbody>
                    </table>
                </div>
            </div>
        }
    }
}

impl DataGrid {
    fn view_th(&self, ctx: &Context<Self>, _idx: usize, col: &str) -> Html {
        let p = ctx.props();
        let is_active = p.sort_column.as_deref() == Some(col);
        let on_sort = p.on_sort.clone();
        let col_name = col.to_string();
        let onclick = Callback::from(move |_| on_sort.emit(col_name.clone()));
        let arrow = if is_active { " ↓" } else { "" };
        html! {
            <th
                class="text-left px-4 py-3 font-display font-semibold text-[13px] text-text whitespace-nowrap cursor-pointer hover:bg-background"
                {onclick}
            >
                { col }{ arrow }
            </th>
        }
    }

    fn view_row(&self, row: &[CellValue], _cols: usize) -> Html {
        html! {
            <tr class="border-b border-border last:border-b-0 hover:bg-background/60">
                { for row.iter().map(view_cell) }
            </tr>
        }
    }

    fn view_empty(&self, cols: usize) -> Html {
        html! {
            <tr>
                <td colspan={cols.to_string()} class="px-4 py-10 text-center text-text-secondary">
                    { "No rows" }
                </td>
            </tr>
        }
    }
}

fn view_cell(cell: &CellValue) -> Html {
    match cell {
        CellValue::Null => html! {
            <td class="px-4 py-2.5 text-neutral italic font-mono text-[13px]">{ "NULL" }</td>
        },
        CellValue::Bool(b) => html! {
            <td class="px-4 py-2.5 font-mono text-[13px]">{ if *b { "true" } else { "false" } }</td>
        },
        CellValue::Int(n) => html! {
            <td class="px-4 py-2.5 font-mono text-[13px] text-right">{ n.to_string() }</td>
        },
        CellValue::Float(f) => html! {
            <td class="px-4 py-2.5 font-mono text-[13px] text-right">{ f.to_string() }</td>
        },
        CellValue::String(s) => {
            let display = if s.chars().count() > 80 {
                let mut truncated: String = s.chars().take(80).collect();
                truncated.push('…');
                truncated
            } else {
                s.clone()
            };
            html! {
                <td class="px-4 py-2.5 max-w-[400px] truncate" title={s.clone()}>{ display }</td>
            }
        }
        CellValue::Bytes { base64 } => html! {
            <td class="px-4 py-2.5 font-mono text-[12px] text-text-secondary max-w-[200px] truncate" title={base64.clone()}>
                { format!("0x{} bytes", base64.len()) }
            </td>
        },
        CellValue::Json(v) => {
            let s = v.to_string();
            html! {
                <td class="px-4 py-2.5 font-mono text-[12px] max-w-[400px] truncate" title={s.clone()}>{ s }</td>
            }
        }
    }
}
