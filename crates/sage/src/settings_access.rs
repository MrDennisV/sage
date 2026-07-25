//! Settings that transports expose as commands of their own rather than as
//! API endpoints. The logic lives here so the desktop commands and the browser
//! dispatch call the same code instead of each carrying a copy.

use chia_sdk_utils::Address;
use sage_config::{NetworkConfig, Wallet, WalletDefaults};
use sage_database::SqlExecutor;

use crate::{Error, Result, Sage};

impl<E: SqlExecutor> Sage<E> {
    /// Whether an address is well formed and belongs to the active network.
    pub fn is_valid_address(&self, address: &str) -> bool {
        Address::decode(address).is_ok_and(|decoded| decoded.prefix == self.network().prefix())
    }

    pub fn network_config(&self) -> NetworkConfig {
        self.config.network.clone()
    }

    pub fn wallet_defaults(&self) -> WalletDefaults {
        self.wallet_config.defaults
    }

    /// The stored settings for a wallet, whether or not it is the active one.
    pub fn wallet_config_of(&self, fingerprint: u32) -> Option<Wallet> {
        self.wallet_config
            .wallets
            .iter()
            .find(|wallet| wallet.fingerprint == fingerprint)
            .cloned()
    }

    /// Moves a wallet within the list, which is the order the interface shows.
    pub fn move_wallet(&mut self, fingerprint: u32, index: u32) -> Result<()> {
        let current = self
            .wallet_config
            .wallets
            .iter()
            .position(|wallet| wallet.fingerprint == fingerprint)
            .ok_or(Error::UnknownFingerprint)?;

        let wallet = self.wallet_config.wallets.remove(current);
        let index = (index as usize).min(self.wallet_config.wallets.len());
        self.wallet_config.wallets.insert(index, wallet);
        self.save_config()?;

        Ok(())
    }

    /// Records the network to use by default. Reconnecting is the caller's
    /// job, since how a wallet reaches the chain differs per transport.
    pub fn select_network(&mut self, name: String) -> Result<()> {
        self.config.network.default_network = name;
        self.save_config()?;

        Ok(())
    }

    /// Records the network a single wallet uses, overriding the default.
    pub fn override_wallet_network(
        &mut self,
        fingerprint: u32,
        name: Option<String>,
    ) -> Result<()> {
        let wallet = self
            .wallet_config
            .wallets
            .iter_mut()
            .find(|wallet| wallet.fingerprint == fingerprint)
            .ok_or(Error::UnknownFingerprint)?;

        wallet.network = name;
        self.save_config()?;

        Ok(())
    }
}
