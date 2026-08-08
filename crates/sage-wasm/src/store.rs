use base64::{Engine, prelude::BASE64_STANDARD};
use sage::{Error, KvStore, Result};
use wasm_bindgen::prelude::*;

// The storage bridge implemented by the service worker. It preloads a
// snapshot from chrome.storage and write-through caches, so both calls are
// synchronous. Values cross the boundary base64-encoded; a missing entry
// reads as null.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = kvRead, catch)]
    fn kv_read(name: &str) -> std::result::Result<Option<String>, JsValue>;

    #[wasm_bindgen(js_name = kvWrite, catch)]
    fn kv_write(name: &str, base64: &str) -> std::result::Result<(), JsValue>;
}

fn js_error(error: &JsValue) -> Error {
    Error::Store(error.as_string().unwrap_or_else(|| format!("{error:?}")))
}

/// A [`KvStore`] over the service worker's storage bridge, holding Sage's
/// config and keychain files.
#[derive(Debug, Clone, Copy)]
pub struct BrowserStore;

impl KvStore for BrowserStore {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(encoded) = kv_read(name).map_err(|error| js_error(&error))? else {
            return Ok(None);
        };

        let bytes = BASE64_STANDARD
            .decode(encoded)
            .map_err(|error| Error::Store(format!("invalid base64 in {name}: {error}")))?;

        Ok(Some(bytes))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        kv_write(name, &BASE64_STANDARD.encode(bytes)).map_err(|error| js_error(&error))
    }
}
