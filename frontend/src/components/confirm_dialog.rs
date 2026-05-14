use yew::prelude::*;

use crate::components::button::{Button, ButtonVariant};
use crate::components::modal::Modal;
use crate::theme;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub title: AttrValue,
    pub body: AttrValue,
    #[prop_or(AttrValue::from("Confirm"))]
    pub confirm_label: AttrValue,
    pub on_confirm: Callback<()>,
    pub on_cancel: Callback<()>,
    #[prop_or(false)]
    pub busy: bool,
}

pub struct ConfirmDialog;

impl Component for ConfirmDialog {
    type Message = ();
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = ctx.props();
        let on_close = p.on_cancel.clone();
        let on_close_cb = Callback::from(move |_| on_close.emit(()));
        let on_cancel = p.on_cancel.clone();
        let on_cancel_cb = Callback::from(move |_| on_cancel.emit(()));
        let on_confirm = p.on_confirm.clone();
        let on_confirm_cb = Callback::from(move |_| on_confirm.emit(()));

        html! {
            <Modal title={p.title.clone()} on_close={on_close_cb} width_class={AttrValue::from("max-w-md")}>
                <p class="text-sm text-text mb-6">{ p.body.clone() }</p>
                <div class="flex justify-end gap-2">
                    <Button variant={ButtonVariant::Secondary} disabled={p.busy} onclick={on_cancel_cb}>
                        { Html::from("Cancel") }
                    </Button>
                    <button
                        class={theme::BTN_DESTRUCTIVE}
                        type="button"
                        disabled={p.busy}
                        onclick={on_confirm_cb}
                    >
                        { p.confirm_label.clone() }
                    </button>
                </div>
            </Modal>
        }
    }
}
