use std::{cell::RefCell, path::PathBuf, rc::Rc};

use sage::Sage;
use sage_database::Database;
use sage_keychain::Keychain;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{BrowserExecutor, BrowserStore, js_error};

/// The one Sage instance for this wasm module.
pub(crate) type SageCell = Rc<RefCell<Option<Sage<BrowserExecutor>>>>;

thread_local! {
    static SAGE: SageCell = Rc::new(RefCell::new(None));
}

/// A handle to the Sage instance. It is handed out as an `Rc` so async
/// exports can keep the borrow alive across a suspension point, which
/// endpoints that talk to the network need.
pub(crate) fn sage_cell() -> SageCell {
    SAGE.with(Rc::clone)
}

pub(crate) fn not_initialized() -> JsValue {
    JsValue::from_str("sage is not initialized; call sage_init first")
}

/// The module is single threaded, so the only way the instance is already
/// borrowed is a command arriving while another one is suspended.
pub(crate) fn already_busy() -> JsValue {
    JsValue::from_str("sage is already handling a command; retry once it finishes")
}

/// Creates the Sage instance over the browser bridges, loading the keychain
/// and config files from the store. An empty `network_id` keeps the stored
/// (or default) network. The database is selected separately, since each
/// wallet and network pair has its own database.
#[wasm_bindgen]
pub fn sage_init(network_id: &str) -> Result<(), JsValue> {
    let mut sage = Sage::with_store(Box::new(BrowserStore), PathBuf::new());

    load_stored_state(&mut sage, network_id).map_err(js_error)?;

    sage_cell().replace(Some(sage));

    Ok(())
}

/// The wallet and network the stored config points at, so the caller can
/// select the matching database before logging in.
#[derive(Debug, Serialize)]
pub struct Session {
    pub fingerprint: Option<u32>,
    pub network_id: String,
}

#[wasm_bindgen]
pub fn sage_session() -> Result<String, JsValue> {
    let cell = sage_cell();
    let guard = cell.borrow();
    let sage = guard.as_ref().ok_or_else(not_initialized)?;

    serde_json::to_string(&Session {
        fingerprint: sage.config.global.fingerprint,
        network_id: sage.network_id(),
    })
    .map_err(js_error)
}

/// Applies the schema migrations to the currently selected database. The
/// caller selects which database is active before calling this.
#[wasm_bindgen]
pub async fn sage_prepare_database() -> Result<(), JsValue> {
    sage_database::run_migrations(&BrowserExecutor)
        .await
        .map_err(js_error)
}

/// Logs into a wallet by fingerprint, building the wallet over the browser
/// database. Runs the same data migrations native performs on wallet switch.
#[wasm_bindgen]
pub async fn sage_login(fingerprint: u32) -> Result<(), JsValue> {
    let cell = sage_cell();

    // Release the borrow before awaiting so a command arriving in the
    // meantime isn't rejected as busy.
    let ticker = {
        let guard = cell.borrow();
        let sage = guard.as_ref().ok_or_else(not_initialized)?;

        if !sage.keychain.contains(fingerprint) {
            return Err(js_error(sage::Error::UnknownFingerprint));
        }

        sage.network().ticker.clone()
    };

    Database::from_executor(BrowserExecutor)
        .run_rust_migrations(ticker)
        .await
        .map_err(js_error)?;

    let mut guard = cell.borrow_mut();
    let sage = guard.as_mut().ok_or_else(not_initialized)?;

    sage.config.global.fingerprint = Some(fingerprint);
    sage.save_config().map_err(js_error)?;

    sage.login_with_database(fingerprint, Database::from_executor(BrowserExecutor))
        .map_err(js_error)
}

/// Mirrors the native `setup_keys`/`setup_config`, minus the legacy config
/// migrations which never apply to a fresh browser store. Missing files keep
/// the defaults already set by `Sage::with_store` and are written back.
fn load_stored_state(sage: &mut Sage<BrowserExecutor>, network_id: &str) -> sage::Result<()> {
    if let Some(bytes) = sage.store.read("keys.bin")? {
        sage.keychain = Keychain::from_bytes(&bytes)?;
    } else {
        sage.save_keychain()?;
    }

    let mut save = false;

    if let Some(text) = read_text(sage, "config.toml")? {
        sage.config = toml::from_str(&text)?;
    } else {
        save = true;
    }

    if let Some(text) = read_text(sage, "wallets.toml")? {
        sage.wallet_config = toml::from_str(&text)?;
    } else {
        save = true;
    }

    if let Some(text) = read_text(sage, "networks.toml")? {
        sage.network_list = toml::from_str(&text)?;
    } else {
        save = true;
    }

    if !network_id.is_empty() {
        sage.config.network.default_network = network_id.to_string();
    }

    if save {
        sage.save_config()?;
    }

    Ok(())
}

fn read_text(sage: &Sage<BrowserExecutor>, name: &str) -> sage::Result<Option<String>> {
    sage.store
        .read(name)?
        .map(|bytes| {
            String::from_utf8(bytes)
                .map_err(|error| sage::Error::Store(format!("invalid utf-8 in {name}: {error}")))
        })
        .transpose()
}
