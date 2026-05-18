//! Runtime dark-mode controller.
//!
//! Owns the user's theme preference (Light / Dark / System), persists it to
//! `localStorage`, and listens to `prefers-color-scheme` so the `System`
//! preference live-tracks the OS. The current [`ThemeCtx`] is distributed to
//! any descendant via Yew's `ContextProvider`.

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MediaQueryList, MediaQueryListEvent, window};
use yew::prelude::*;

const STORAGE_KEY: &str = "mysqlview-theme";
const MEDIA_QUERY: &str = "(prefers-color-scheme: dark)";

/// User's explicit preference. Cycles `Light → Dark → System → Light`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemePref {
    Light,
    Dark,
    System,
}

impl ThemePref {
    fn as_storage_str(self) -> &'static str {
        match self {
            ThemePref::Light => "light",
            ThemePref::Dark => "dark",
            ThemePref::System => "system",
        }
    }

    fn from_storage_str(s: &str) -> Option<Self> {
        match s {
            "light" => Some(ThemePref::Light),
            "dark" => Some(ThemePref::Dark),
            "system" => Some(ThemePref::System),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            ThemePref::Light => ThemePref::Dark,
            ThemePref::Dark => ThemePref::System,
            ThemePref::System => ThemePref::Light,
        }
    }
}

/// Theme actually applied to `<html data-theme>`. Always Light or Dark
/// — `System` is resolved against the live OS preference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectiveTheme {
    Light,
    Dark,
}

impl EffectiveTheme {
    fn as_attr(self) -> &'static str {
        match self {
            EffectiveTheme::Light => "light",
            EffectiveTheme::Dark => "dark",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ThemeCtx {
    pub pref: ThemePref,
    pub effective: EffectiveTheme,
    pub cycle: Callback<()>,
}

pub enum Msg {
    Cycle,
    SystemChanged(bool),
}

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub children: Html,
}

pub struct ThemeProvider {
    pref: ThemePref,
    system_dark: bool,
    _media_handle: Option<MediaListenerHandle>,
}

impl Component for ThemeProvider {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let pref = read_saved().unwrap_or(ThemePref::System);
        let system_dark = read_system();
        apply_attribute(compute_effective(pref, system_dark));

        let link = ctx.link().clone();
        let media_handle = attach_media_listener(move |is_dark| {
            link.send_message(Msg::SystemChanged(is_dark));
        });

        Self {
            pref,
            system_dark,
            _media_handle: media_handle,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Cycle => {
                self.pref = self.pref.next();
                write_saved(self.pref);
                apply_attribute(compute_effective(self.pref, self.system_dark));
                true
            }
            Msg::SystemChanged(is_dark) => {
                self.system_dark = is_dark;
                if matches!(self.pref, ThemePref::System) {
                    apply_attribute(compute_effective(self.pref, self.system_dark));
                    true
                } else {
                    false
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let value = ThemeCtx {
            pref: self.pref,
            effective: compute_effective(self.pref, self.system_dark),
            cycle: ctx.link().callback(|_| Msg::Cycle),
        };
        html! {
            <ContextProvider<ThemeCtx> context={value}>
                { ctx.props().children.clone() }
            </ContextProvider<ThemeCtx>>
        }
    }
}

fn compute_effective(pref: ThemePref, system_dark: bool) -> EffectiveTheme {
    match pref {
        ThemePref::Light => EffectiveTheme::Light,
        ThemePref::Dark => EffectiveTheme::Dark,
        ThemePref::System => {
            if system_dark {
                EffectiveTheme::Dark
            } else {
                EffectiveTheme::Light
            }
        }
    }
}

fn read_saved() -> Option<ThemePref> {
    let win = window()?;
    let storage = win.local_storage().ok().flatten()?;
    let s = storage.get_item(STORAGE_KEY).ok().flatten()?;
    ThemePref::from_storage_str(&s)
}

fn read_system() -> bool {
    window()
        .and_then(|w| w.match_media(MEDIA_QUERY).ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

fn write_saved(p: ThemePref) {
    if let Some(win) = window()
        && let Ok(Some(storage)) = win.local_storage()
    {
        let _ = storage.set_item(STORAGE_KEY, p.as_storage_str());
    }
}

fn apply_attribute(eff: EffectiveTheme) {
    let _ = window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .map(|el| el.set_attribute("data-theme", eff.as_attr()));
}

/// RAII guard that detaches the matchMedia listener on drop.
struct MediaListenerHandle {
    mql: MediaQueryList,
    closure: Closure<dyn FnMut(MediaQueryListEvent)>,
}

impl Drop for MediaListenerHandle {
    fn drop(&mut self) {
        let _ = self
            .mql
            .remove_event_listener_with_callback("change", self.closure.as_ref().unchecked_ref());
    }
}

fn attach_media_listener<F>(mut on_change: F) -> Option<MediaListenerHandle>
where
    F: FnMut(bool) + 'static,
{
    let mql = window()?.match_media(MEDIA_QUERY).ok().flatten()?;
    let closure: Closure<dyn FnMut(MediaQueryListEvent)> =
        Closure::new(move |e: MediaQueryListEvent| on_change(e.matches()));
    mql.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
        .ok()?;
    Some(MediaListenerHandle { mql, closure })
}
