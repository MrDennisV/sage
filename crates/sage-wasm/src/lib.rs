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
