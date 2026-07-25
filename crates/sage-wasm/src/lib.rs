//! WebAssembly entry point for running the Sage wallet core in a browser.
//!
//! The service worker provides the JS side of the bridges (SQL engine and
//! storage); this crate wires them into the portable Sage core.

#![cfg(target_arch = "wasm32")]

mod bootstrap;
mod dispatch;
mod executor;
mod store;
mod sync;

pub use bootstrap::*;
pub use dispatch::*;
pub use executor::*;
pub use store::*;
pub use sync::*;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Converts any displayable error into a JS string error.
pub(crate) fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Reports a wallet error the same way the desktop commands do, so the
/// interface can tell an unauthorized request from a missing record instead of
/// only seeing a message.
pub(crate) fn sage_error(error: sage::Error) -> JsValue {
    #[derive(serde::Serialize)]
    struct ApiError {
        kind: sage_api::ErrorKind,
        reason: String,
    }

    let payload = ApiError {
        kind: error.kind(),
        reason: error.to_string(),
    };

    serde_json::to_string(&payload).map_or_else(|_| js_error(error), |json| JsValue::from_str(&json))
}
