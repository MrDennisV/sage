use std::{path::PathBuf, sync::Arc};

use chia_protocol::Bytes32;
use chia_sdk_utils::Address;
use sage_api::Unit;
use sage_config::{Config, Network, NetworkList, WalletConfig};
use sage_database::SqlExecutor;
use sage_keychain::Keychain;
use sage_wallet::Wallet;

use crate::{Error, KvStore, Result};

cfg_if::cfg_if! {
    if #[cfg(feature = "native")] {
        use sage_database::SqlxExecutor;
        use sage_wallet::{PeerState, SyncCommand};
        use tokio::sync::{Mutex, mpsc};

        #[derive(Debug)]
        pub struct Sage<E: SqlExecutor = SqlxExecutor> {
            pub path: PathBuf,
            pub store: Box<dyn KvStore>,
            pub config: Config,
            pub wallet_config: WalletConfig,
            pub network_list: NetworkList,
            pub keychain: Keychain,
            pub wallet: Option<Arc<Wallet<E>>>,
            pub peer_state: Arc<Mutex<PeerState>>,
            pub command_sender: mpsc::Sender<SyncCommand>,
            pub unit: Unit,
            pub test: bool,
        }
    } else {
        #[derive(Debug)]
        pub struct Sage<E: SqlExecutor> {
            pub path: PathBuf,
            pub store: Box<dyn KvStore>,
            pub config: Config,
            pub wallet_config: WalletConfig,
            pub network_list: NetworkList,
            pub keychain: Keychain,
            pub wallet: Option<Arc<Wallet<E>>>,
            pub unit: Unit,
            pub test: bool,
        }
    }
}

impl<E: SqlExecutor> Sage<E> {
    #[cfg(not(feature = "native"))]
    pub fn with_store(store: Box<dyn KvStore>, path: PathBuf) -> Self {
        Self {
            path,
            store,
            config: Config::default(),
            wallet_config: WalletConfig::default(),
            network_list: NetworkList::default(),
            keychain: Keychain::default(),
            wallet: None,
            unit: sage_api::XCH.clone(),
            test: false,
        }
    }

    pub fn parse_address(&self, input: String) -> Result<Bytes32> {
        let address = Address::decode(&input)?;

        if address.prefix != self.network().prefix() {
            return Err(Error::AddressPrefix(address.prefix));
        }

        Ok(address.puzzle_hash)
    }

    pub fn wallet_config(&self) -> Option<&sage_config::Wallet> {
        self.config.global.fingerprint.and_then(|fingerprint| {
            self.wallet_config
                .wallets
                .iter()
                .find(|w| w.fingerprint == fingerprint)
        })
    }

    pub fn network(&self) -> &Network {
        if let Some(wallet) = self.wallet_config()
            && let Some(network) = &wallet.network
        {
            return self
                .network_list
                .by_name(network)
                .expect("network not found");
        }

        self.network_list
            .by_name(&self.config.network.default_network)
            .expect("network not found")
    }

    pub fn network_id(&self) -> String {
        self.network().network_id()
    }

    pub fn wallet(&self) -> Result<Arc<Wallet<E>>> {
        let Some(fingerprint) = self.config.global.fingerprint else {
            return Err(Error::NotLoggedIn);
        };

        if !self.keychain.contains(fingerprint) {
            return Err(Error::UnknownFingerprint);
        }

        let wallet = self.wallet.as_ref().ok_or(Error::NotLoggedIn)?;

        if wallet.fingerprint != fingerprint {
            return Err(Error::NotLoggedIn);
        }

        Ok(wallet.clone())
    }

    pub fn save_config(&self) -> Result<()> {
        let config = toml::to_string_pretty(&self.config)?;
        self.store.write("config.toml", config.as_bytes())?;
        let wallet_config = toml::to_string_pretty(&self.wallet_config)?;
        self.store.write("wallets.toml", wallet_config.as_bytes())?;
        let network_list = toml::to_string_pretty(&self.network_list)?;
        self.store.write("networks.toml", network_list.as_bytes())?;
        Ok(())
    }

    pub fn save_keychain(&self) -> Result<()> {
        self.store.write("keys.bin", &self.keychain.to_bytes()?)?;
        Ok(())
    }
}
