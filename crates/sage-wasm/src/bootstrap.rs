use std::{cell::RefCell, path::PathBuf};

use sage::Sage;
use sage_database::Database;
use sage_keychain::Keychain;
use wasm_bindgen::prelude::*;

use crate::{BrowserExecutor, BrowserStore, js_error};

thread_local! {
    /// The single Sage instance for this wasm module. Exported async fns must
    /// never hold a borrow across an await; clone what they need out first.
    pub(crate) static SAGE: RefCell<Option<Sage<BrowserExecutor>>> = const { RefCell::new(None) };
}

pub(crate) fn not_initialized() -> JsValue {
    JsValue::from_str("sage is not initialized; call sage_init first")
}

/// Creates the Sage instance over the browser bridges: runs the database
/// migrations, then loads the keychain and config files from the store. An
/// empty `network_id` keeps the stored (or default) network.
#[wasm_bindgen]
pub async fn sage_init(network_id: String) -> Result<(), JsValue> {
    let mut sage = Sage::with_store(Box::new(BrowserStore), PathBuf::new());

    sage_database::run_migrations(&BrowserExecutor)
        .await
        .map_err(js_error)?;

    load_stored_state(&mut sage, &network_id).map_err(js_error)?;

    SAGE.with(|cell| cell.replace(Some(sage)));

    Ok(())
}

/// Logs into a wallet by fingerprint, building the wallet over the browser
/// database. Runs the same data migrations native performs on wallet switch.
#[wasm_bindgen]
pub async fn sage_login(fingerprint: u32) -> Result<(), JsValue> {
    // Clone the ticker out before awaiting; a RefCell borrow must not be
    // held across a suspension point.
    let ticker = SAGE.with(|cell| {
        let guard = cell.borrow();
        let sage = guard.as_ref().ok_or_else(not_initialized)?;

        if !sage.keychain.contains(fingerprint) {
            return Err(js_error(sage::Error::UnknownFingerprint));
        }

        Ok(sage.network().ticker.clone())
    })?;

    Database::from_executor(BrowserExecutor)
        .run_rust_migrations(ticker)
        .await
        .map_err(js_error)?;

    SAGE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let sage = guard.as_mut().ok_or_else(not_initialized)?;

        sage.config.global.fingerprint = Some(fingerprint);
        sage.save_config().map_err(js_error)?;

        sage.login_with_database(fingerprint, Database::from_executor(BrowserExecutor))
            .map_err(js_error)
    })
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
