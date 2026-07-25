use std::cell::Cell;

use sage::Sage;
use sage_api::{
    DeleteDatabase, DeleteDatabaseResponse, DeleteKey, DeleteKeyResponse, GetPeersResponse,
    ImportKey, ImportKeyResponse, Login, LoginResponse, Logout, LogoutResponse, Resync,
    SetChangeAddress, SetChangeAddressResponse, SetNetwork, SetNetworkOverride,
    SetNetworkOverrideResponse, SetNetworkResponse,
};
use sage_api_macro::impl_endpoints_portable;
use sage_database::Database;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

use crate::{
    BrowserExecutor,
    bootstrap::{already_busy, not_initialized, require_mounted, sage_cell},
    js_error, sage_error,
};

thread_local! {
    /// Set while a command is running. The instance is moved out of its cell
    /// for the duration, so this is what tells a busy module apart from an
    /// uninitialized one.
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

/// Dispatches an API command to the Sage instance. `payload` is the
/// JSON-encoded request and the result is the JSON-encoded response. Errors,
/// including unknown commands, are returned as plain string `JsValue`s.
#[wasm_bindgen]
pub async fn sage_handle(command: String, payload: String) -> Result<String, JsValue> {
    if BUSY.get() {
        return Err(already_busy());
    }

    // The instance is moved out of its cell for the duration of the command,
    // since a borrow must not be held across the network round trips that
    // submitting endpoints make.
    let cell = sage_cell();
    let mut sage = cell.borrow_mut().take().ok_or_else(not_initialized)?;

    BUSY.set(true);
    let result = handle(&mut sage, &command, &payload).await;
    *cell.borrow_mut() = Some(sage);
    BUSY.set(false);

    result
}

async fn handle(
    sage: &mut Sage<BrowserExecutor>,
    command: &str,
    payload: &str,
) -> Result<String, JsValue> {
    if let Some(response) = session_command(sage, command, payload)? {
        return Ok(response);
    }

    if let Some(response) = browser_command(sage, command, payload).await? {
        return Ok(response);
    }

    if let Some(response) = wallet_connect_command(sage, command, payload).await? {
        return Ok(response);
    }

    impl_endpoints_portable! {
        (
            generate_mnemonic, rename_key, set_wallet_emoji, get_key, get_secret_key, get_keys,
            get_version, get_sync_status, check_address, get_derivations, get_are_coins_spendable,
            get_spendable_coin_count, get_coins_by_ids, get_coins, get_cats, get_all_cats,
            get_token, get_dids, get_minter_did_ids, get_options, get_option,
            get_pending_transactions, get_transaction, get_transactions, get_nft_collections,
            get_nft_collection, get_nfts, get_nft, get_nft_icon, get_nft_thumbnail, get_nft_data,
            is_asset_owned, send_xch, bulk_send_xch, combine, split, auto_combine_xch,
            auto_combine_cat, issue_cat, send_cat, bulk_send_cat, multi_send, create_did,
            bulk_mint_nfts, transfer_nfts, add_nft_uri, assign_nfts_to_did, transfer_dids,
            normalize_dids, mint_option, transfer_options, exercise_options, finalize_clawback,
            create_transaction, sign_coin_spends, view_coin_spends, submit_transaction, make_offer,
            take_offer, combine_offers, view_offer, import_offer, get_offers, get_offers_for_asset,
            get_offer, delete_offer, cancel_offer, cancel_offers, get_networks, get_network,
            set_delta_sync, set_delta_sync_override, set_network_api_url, update_cat, resync_cat,
            update_did, update_option,
            update_nft, update_nft_collection, redownload_nft, increase_derivation_index
        )
        Ok(match command {
            (repeat endpoint_string => {
                let req: sage_api::Endpoint = decode(payload)?;
                let res = sage.endpoint(req) maybe_await .map_err(sage_error)?;
                encode(&res)?
            })
            _ => return Err(JsValue::from_str(&format!("unsupported command: {command}"))),
        })
    }
}

/// Commands whose browser implementation differs from the desktop one, because
/// what they act on is a file there and something else here: the mounted
/// database, the key-value store, or a plain HTTPS request.
async fn browser_command(
    sage: &mut Sage<BrowserExecutor>,
    command: &str,
    payload: &str,
) -> Result<Option<String>, JsValue> {
    let response = match command {
        "resync" => {
            let req: Resync = decode(payload)?;
            require_mounted(req.fingerprint)?;

            let res = sage
                .resync_database(&Database::from_executor(BrowserExecutor), req)
                .await
                .map_err(sage_error)?;

            encode(&res)?
        }
        "delete_database" => {
            let req: DeleteDatabase = decode(payload)?;
            require_mounted(req.fingerprint)?;

            // Desktop removes the network's SQLite file. Here the database is
            // mounted rather than opened by path, so emptying it means dropping
            // the schema and building it again.
            sage_database::drop_schema(&BrowserExecutor)
                .await
                .map_err(|error| sage_error(error.into()))?;

            sage_database::run_migrations(&BrowserExecutor)
                .await
                .map_err(|error| sage_error(error.into()))?;

            encode(&DeleteDatabaseResponse {})?
        }
        "set_change_address" => {
            let req: SetChangeAddress = decode(payload)?;
            sage.set_change_address_config(req).map_err(sage_error)?;

            // The change address is baked into the wallet when it is built, so
            // rebuild it the way logging in does for the new one to take hold.
            if let Some(fingerprint) = sage.config.global.fingerprint {
                sage.login_with_database(fingerprint, Database::from_executor(BrowserExecutor))
                    .map_err(sage_error)?;
            }

            encode(&SetChangeAddressResponse {})?
        }
        // Themes are stored per key rather than per directory, so these are not
        // part of the generated endpoint set even though the requests are.
        "get_user_theme" => encode(&sage.get_user_theme(decode(payload)?).map_err(sage_error)?)?,
        "get_user_themes" => encode(&sage.get_user_themes(decode(payload)?).map_err(sage_error)?)?,
        "save_user_theme" => encode(
            &sage
                .save_user_theme(decode(payload)?)
                .await
                .map_err(sage_error)?,
        )?,
        "delete_user_theme" => encode(
            &sage
                .delete_user_theme(decode(payload)?)
                .map_err(sage_error)?,
        )?,
        "download_cni_offercode" => {
            let req: OfferCodeRequest = decode(payload)?;

            encode(
                &sage::download_cni_offercode(req.code)
                    .await
                    .map_err(sage_error)?,
            )?
        }
        _ => return Ok(None),
    };

    Ok(Some(response))
}

/// The endpoints dApps reach through `window.chia`. They are not part of the
/// generated endpoint set, so they are dispatched by hand. Sending a bundle
/// straight to the network is missing on purpose: it broadcasts through the
/// peer pool, which the browser build does not have.
async fn wallet_connect_command(
    sage: &Sage<BrowserExecutor>,
    command: &str,
    payload: &str,
) -> Result<Option<String>, JsValue> {
    let response = match command {
        "filter_unlocked_coins" => encode(
            &sage
                .filter_unlocked_coins(decode(payload)?)
                .await
                .map_err(sage_error)?,
        )?,
        "get_asset_coins" => encode(
            &sage
                .get_asset_coins(decode(payload)?)
                .await
                .map_err(sage_error)?,
        )?,
        "sign_message_with_public_key" => encode(
            &sage
                .sign_message_with_public_key(decode(payload)?)
                .await
                .map_err(sage_error)?,
        )?,
        "sign_message_by_address" => encode(
            &sage
                .sign_message_by_address(decode(payload)?)
                .await
                .map_err(sage_error)?,
        )?,
        _ => return Ok(None),
    };

    Ok(Some(response))
}

/// Commands that change which wallet or network is active. They only touch the
/// config here; the service worker selects the matching database afterwards
/// and calls `sage_login`, which is what actually builds the wallet.
fn session_command(
    sage: &mut Sage<BrowserExecutor>,
    command: &str,
    payload: &str,
) -> Result<Option<String>, JsValue> {
    let response = match command {
        "login" => {
            let req: Login = decode(payload)?;
            sage.config.global.fingerprint = Some(req.fingerprint);
            sage.save_config().map_err(sage_error)?;
            encode(&LoginResponse {})?
        }
        "logout" => {
            let _req: Logout = decode(payload)?;
            sage.config.global.fingerprint = None;
            sage.save_config().map_err(sage_error)?;
            sage.wallet = None;
            encode(&LogoutResponse {})?
        }
        "import_key" => {
            let req: ImportKey = decode(payload)?;
            // Addresses aren't derived here because the wallet's database
            // isn't selected yet; the first sync fills them in.
            let (fingerprint, _master_sk, _master_pk) = sage.add_key(&req).map_err(sage_error)?;
            encode(&ImportKeyResponse { fingerprint })?
        }
        "delete_key" => {
            let req: DeleteKey = decode(payload)?;

            // The wallet's databases are removed by the caller, so drop the
            // handle to them here.
            if sage.config.global.fingerprint == Some(req.fingerprint) {
                sage.wallet = None;
            }

            sage.remove_key(req.fingerprint).map_err(sage_error)?;
            encode(&DeleteKeyResponse {})?
        }
        "set_network" => {
            let req: SetNetwork = decode(payload)?;
            sage.select_network(req.name).map_err(sage_error)?;
            encode(&SetNetworkResponse {})?
        }
        "set_network_override" => {
            let req: SetNetworkOverride = decode(payload)?;
            sage.override_wallet_network(req.fingerprint, req.name)
                .map_err(sage_error)?;
            encode(&SetNetworkOverrideResponse {})?
        }
        // The wallet is rebuilt by the service worker's `sage_login` call, so
        // there is nothing to do here. Matches the unit the native command
        // returns.
        "switch_wallet" | "initialize" => encode(&())?,
        "validate_address" => {
            let req: AddressRequest = decode(payload)?;
            encode(&sage.is_valid_address(&req.address))?
        }
        "move_key" => {
            let req: MoveKeyRequest = decode(payload)?;
            sage.move_wallet(req.fingerprint, req.index)
                .map_err(sage_error)?;
            encode(&())?
        }
        // Config readers. These are hand-written Tauri commands rather than
        // API endpoints, so they aren't part of the generated dispatch.
        "network_config" => encode(&sage.network_config())?,
        "default_wallet_config" => encode(&sage.wallet_defaults())?,
        "wallet_config" => {
            let req: WalletConfigRequest = decode(payload)?;
            encode(&sage.wallet_config_of(req.fingerprint))?
        }
        // There are no peer connections in the HTTP model; every request goes
        // straight to the Coinset API.
        "get_peers" => encode(&GetPeersResponse { peers: Vec::new() })?,
        _ => return Ok(None),
    };

    Ok(Some(response))
}

// These commands take bare arguments rather than a request struct, matching
// their Tauri signatures.

#[derive(Debug, Deserialize)]
struct WalletConfigRequest {
    fingerprint: u32,
}

#[derive(Debug, Deserialize)]
struct AddressRequest {
    address: String,
}

#[derive(Debug, Deserialize)]
struct MoveKeyRequest {
    fingerprint: u32,
    index: u32,
}

#[derive(Debug, Deserialize)]
struct OfferCodeRequest {
    code: String,
}

fn decode<T: DeserializeOwned>(payload: &str) -> Result<T, JsValue> {
    serde_json::from_str(payload).map_err(js_error)
}

fn encode<T: Serialize>(response: &T) -> Result<String, JsValue> {
    serde_json::to_string(response).map_err(js_error)
}
