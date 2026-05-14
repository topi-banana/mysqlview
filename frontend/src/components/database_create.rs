use mysqlview_types::CreateDatabaseRequest;
use yew::prelude::*;

use crate::api::{self, ApiClientError};
use crate::components::button::{Button, ButtonVariant};
use crate::components::error_banner::ErrorBanner;
use crate::components::input::TextInput;
use crate::components::modal::Modal;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub on_close: Callback<()>,
    pub on_created: Callback<String>,
}

pub enum Msg {
    Name(String),
    Charset(String),
    Collation(String),
    Submit,
    Done(Result<String, ApiClientError>),
}

pub struct DatabaseCreate {
    name: String,
    charset: String,
    collation: String,
    busy: bool,
    error: Option<ApiClientError>,
}

impl Component for DatabaseCreate {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            name: String::new(),
            charset: String::new(),
            collation: String::new(),
            busy: false,
            error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Name(s) => {
                self.name = s;
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
                let name = self.name.trim().to_owned();
                if name.is_empty() {
                    return false;
                }
                let req = CreateDatabaseRequest {
                    name: name.clone(),
                    charset: trim_opt(&self.charset),
                    collation: trim_opt(&self.collation),
                    if_not_exists: false,
                };
                self.busy = true;
                self.error = None;
                ctx.link().send_future(async move {
                    match api::create_database(&req).await {
                        Ok(_) => Msg::Done(Ok(name)),
                        Err(e) => Msg::Done(Err(e)),
                    }
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
                self.error = Some(e);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let on_close = p.on_close.clone();
        let close_cb = Callback::from(move |_| on_close.emit(()));
        let submit = ctx.link().callback(|_| Msg::Submit);
        let cancel = p.on_close.clone();
        let cancel_cb = Callback::from(move |_| cancel.emit(()));

        html! {
            <Modal
                title={AttrValue::from("Create database")}
                on_close={close_cb}
                width_class={AttrValue::from("max-w-lg")}
            >
                <div class="space-y-4">
                    <Field label="Name" hint="Required. Letters, digits, _ and $ only (max 64 chars).">
                        <TextInput
                            value={AttrValue::from(self.name.clone())}
                            placeholder="my_database"
                            oninput={ctx.link().callback(Msg::Name)}
                        />
                    </Field>
                    <Field label="Character set" hint="Optional. Example: utf8mb4">
                        <TextInput
                            value={AttrValue::from(self.charset.clone())}
                            placeholder="utf8mb4"
                            oninput={ctx.link().callback(Msg::Charset)}
                        />
                    </Field>
                    <Field label="Collation" hint="Optional. Example: utf8mb4_0900_ai_ci">
                        <TextInput
                            value={AttrValue::from(self.collation.clone())}
                            placeholder="utf8mb4_0900_ai_ci"
                            oninput={ctx.link().callback(Msg::Collation)}
                        />
                    </Field>
                    if let Some(e) = &self.error {
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
                            disabled={self.busy || self.name.trim().is_empty()}
                            onclick={submit}
                        >
                            { Html::from(if self.busy { "Creating…" } else { "Create" }) }
                        </Button>
                    </div>
                </div>
            </Modal>
        }
    }
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
struct FieldProps {
    label: &'static str,
    #[prop_or_default]
    hint: &'static str,
    children: Children,
}

#[function_component(Field)]
fn field(p: &FieldProps) -> Html {
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
