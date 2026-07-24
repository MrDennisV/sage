//! WebAssembly entry point for running the Sage wallet core in a browser.
//!
//! The service worker provides the JS side of the bridges (SQL engine and
//! storage); this crate wires them into the portable Sage core.

#![cfg(target_arch = "wasm32")]

mod executor;

pub use executor::*;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
