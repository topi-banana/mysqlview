//! Browser-side helpers used by the export and import flows.
//!
//! * [`download_text`] turns an in-memory `String` into a download triggered
//!   by a hidden `<a download="…">` click. Uses the Blob/Url web APIs.
//! * [`read_file_as_text`] returns the contents of a `web_sys::File` as a
//!   `String` (UTF-8 expected; the browser does the decoding).

// Consumers (export buttons, import wizards) land in the next commit.
#![allow(dead_code)]

use js_sys::Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, File, FileReader, HtmlAnchorElement, Url};

/// Trigger a browser download for the given text body. Returns an error if
/// the platform doesn't expose the document / blob APIs (shouldn't happen in
/// a normal browser environment).
pub fn download_text(filename: &str, mime: &str, body: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let parts = Array::new();
    parts.push(&JsValue::from_str(body));
    let bag = BlobPropertyBag::new();
    bag.set_type(mime);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &bag)?;
    let url = Url::create_object_url_with_blob(&blob)?;

    let anchor = document
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.set_attribute("style", "display:none")?;
    let body_el = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body element"))?;
    body_el.append_child(&anchor)?;
    anchor.click();
    body_el.remove_child(&anchor)?;
    Url::revoke_object_url(&url)?;
    Ok(())
}

/// Read a picked `File` as a UTF-8 string. Awaits the underlying FileReader's
/// `load` event without blocking the UI thread.
pub async fn read_file_as_text(file: &File) -> Result<String, String> {
    let reader = FileReader::new().map_err(js_to_string)?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let r1 = reader.clone();
        let r2 = reader.clone();
        let res = resolve.clone();
        let rej_load = reject.clone();
        let rej_err = reject.clone();
        let onload = Closure::<dyn FnMut()>::new(move || match r1.result() {
            Ok(value) => {
                let _ = res.call1(&JsValue::NULL, &value);
            }
            Err(e) => {
                let _ = rej_load.call1(&JsValue::NULL, &e);
            }
        });
        let onerror = Closure::<dyn FnMut()>::new(move || {
            let err = r2
                .error()
                .map(JsValue::from)
                .unwrap_or_else(|| JsValue::from_str("file read failed"));
            let _ = rej_err.call1(&JsValue::NULL, &err);
        });
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        reader.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onload.forget();
        onerror.forget();
    });
    reader.read_as_text(file).map_err(js_to_string)?;
    let value = JsFuture::from(promise).await.map_err(js_to_string)?;
    value
        .as_string()
        .ok_or_else(|| "file contents were not a string".to_owned())
}

fn js_to_string(v: JsValue) -> String {
    v.as_string()
        .or_else(|| v.dyn_ref::<js_sys::Error>().map(|e| e.message().into()))
        .unwrap_or_else(|| format!("{v:?}"))
}
