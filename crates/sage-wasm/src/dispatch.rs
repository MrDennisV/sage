use sage_api_macro::impl_endpoints_portable;
use wasm_bindgen::prelude::*;

use crate::{
    bootstrap::{SAGE, not_initialized},
    js_error,
};

/// Dispatches an API command to the Sage instance. `payload` is the
/// JSON-encoded request and the result is the JSON-encoded response. Errors,
/// including unknown commands, are returned as plain string `JsValue`s.
///
/// Kept async so the JS side always receives a Promise, matching the other
/// exports and leaving room for async endpoints.
#[allow(clippy::unused_async)]
#[wasm_bindgen]
pub async fn sage_handle(command: String, payload: String) -> Result<String, JsValue> {
    // Every portable endpoint is synchronous, so the whole dispatch runs
    // inside a single borrow with no suspension points.
    SAGE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let sage = guard.as_mut().ok_or_else(not_initialized)?;

        impl_endpoints_portable! {
            (generate_mnemonic, rename_key, set_wallet_emoji, get_key, get_secret_key, get_keys)
            Ok(match command.as_str() {
                (repeat endpoint_string => {
                    let req: sage_api::Endpoint = serde_json::from_str(&payload).map_err(js_error)?;
                    let res = sage.endpoint(req) maybe_await .map_err(js_error)?;
                    serde_json::to_string(&res).map_err(js_error)?
                })
                _ => return Err(JsValue::from_str(&format!("unsupported command: {command}"))),
            })
        }
    })
}
