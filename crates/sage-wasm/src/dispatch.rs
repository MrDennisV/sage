use std::cell::Cell;

use chia_sdk_utils::Address;
use sage::Sage;
use sage_api::{
    GetPeersResponse, GetUserThemesResponse, ImportKey, ImportKeyResponse, Login, LoginResponse,
    Logout, LogoutResponse, SetNetwork, SetNetworkOverride, SetNetworkOverrideResponse,
    SetNetworkResponse,
};
use sage_api_macro::impl_endpoints_portable;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

use crate::{
    BrowserExecutor,
    bootstrap::{already_busy, not_initialized, sage_cell},
    js_error,
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
            set_delta_sync, set_delta_sync_override, update_cat, update_did, update_option,
            update_nft, update_nft_collection, redownload_nft, increase_derivation_index
        )
        Ok(match command {
            (repeat endpoint_string => {
                let req: sage_api::Endpoint = decode(payload)?;
                let res = sage.endpoint(req) maybe_await .map_err(js_error)?;
                encode(&res)?
            })
            _ => return Err(JsValue::from_str(&format!("unsupported command: {command}"))),
        })
    }
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
            sage.save_config().map_err(js_error)?;
            encode(&LoginResponse {})?
        }
        "logout" => {
            let _req: Logout = decode(payload)?;
            sage.config.global.fingerprint = None;
            sage.save_config().map_err(js_error)?;
            sage.wallet = None;
            encode(&LogoutResponse {})?
        }
        "import_key" => {
            let req: ImportKey = decode(payload)?;
            // Addresses aren't derived here because the wallet's database
            // isn't selected yet; the first sync fills them in.
            let (fingerprint, _master_sk, _master_pk) = sage.add_key(&req).map_err(js_error)?;
            encode(&ImportKeyResponse { fingerprint })?
        }
        "set_network" => {
            let req: SetNetwork = decode(payload)?;
            sage.config.network.default_network.clone_from(&req.name);
            sage.save_config().map_err(js_error)?;
            encode(&SetNetworkResponse {})?
        }
        "set_network_override" => {
            let req: SetNetworkOverride = decode(payload)?;

            let wallet = sage
                .wallet_config
                .wallets
                .iter_mut()
                .find(|wallet| wallet.fingerprint == req.fingerprint)
                .ok_or_else(|| js_error(sage::Error::UnknownFingerprint))?;

            wallet.network = req.name;
            sage.save_config().map_err(js_error)?;
            encode(&SetNetworkOverrideResponse {})?
        }
        // The wallet is rebuilt by the service worker's `sage_login` call, so
        // there is nothing to do here. Matches the unit the native command
        // returns.
        "switch_wallet" | "initialize" => encode(&())?,
        "validate_address" => {
            let req: AddressRequest = decode(payload)?;
            let valid = Address::decode(&req.address)
                .is_ok_and(|address| address.prefix == sage.network().prefix());
            encode(&valid)?
        }
        "move_key" => {
            let req: MoveKeyRequest = decode(payload)?;

            let index = sage
                .wallet_config
                .wallets
                .iter()
                .position(|wallet| wallet.fingerprint == req.fingerprint)
                .ok_or_else(|| js_error(sage::Error::UnknownFingerprint))?;

            let wallet = sage.wallet_config.wallets.remove(index);
            sage.wallet_config.wallets.insert(req.index as usize, wallet);
            sage.save_config().map_err(js_error)?;
            encode(&())?
        }
        // Config readers. These are hand-written Tauri commands rather than
        // API endpoints, so they aren't part of the generated dispatch.
        "network_config" => encode(&sage.config.network)?,
        "default_wallet_config" => encode(&sage.wallet_config.defaults)?,
        "wallet_config" => {
            let req: WalletConfigRequest = decode(payload)?;
            encode(
                &sage
                    .wallet_config
                    .wallets
                    .iter()
                    .find(|wallet| wallet.fingerprint == req.fingerprint),
            )?
        }
        // User themes are directories on disk, which the browser store has no
        // equivalent for; the built-in themes come from the frontend.
        "get_user_themes" => encode(&GetUserThemesResponse { themes: Vec::new() })?,
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

fn decode<T: DeserializeOwned>(payload: &str) -> Result<T, JsValue> {
    serde_json::from_str(payload).map_err(js_error)
}

fn encode<T: Serialize>(response: &T) -> Result<String, JsValue> {
    serde_json::to_string(response).map_err(js_error)
}
