use chia_bls::Signature;
use chia_protocol::{CoinSpend, SpendBundle};
use chia_sdk_signer::AggSigConstants;
use sage_database::SqlExecutor;
use sage_wallet::{PeerApi, Transaction, insert_transaction};

use crate::{Error, Result, Sage, broadcast};

impl<E: SqlExecutor> Sage<E> {
    pub async fn sign(&self, coin_spends: Vec<CoinSpend>, partial: bool) -> Result<SpendBundle> {
        let wallet = self.wallet()?;

        let (_mnemonic, Some(master_sk)) =
            self.keychain.extract_secrets(wallet.fingerprint, b"")?
        else {
            return Err(Error::NoSigningKey);
        };

        let spend_bundle = wallet
            .sign_transaction(
                SpendBundle::new(coin_spends, Signature::default()),
                &AggSigConstants::new(self.network().agg_sig_me()),
                master_sk,
                partial,
            )
            .await?;

        Ok(spend_bundle)
    }

    pub(crate) async fn submit(&self, spend_bundle: SpendBundle) -> Result<()> {
        // Checked up front so a missing wallet is still reported ahead of a
        // missing peer.
        self.wallet()?;

        let peer = self.acquire_peer().await.ok_or(Error::NoPeers)?;

        self.submit_with_peer(&peer, spend_bundle).await
    }

    /// Records a signed transaction as pending against a specific peer, so it
    /// shows up in the wallet until a later sync confirms or fails it.
    pub async fn submit_with_peer(
        &self,
        peer: &impl PeerApi,
        spend_bundle: SpendBundle,
    ) -> Result<()> {
        let wallet = self.wallet()?;

        broadcast(peer, &spend_bundle).await?;

        let subscriptions = insert_transaction(
            &wallet.db,
            peer,
            wallet.genesis_challenge,
            spend_bundle.name(),
            Transaction::from_coin_spends(spend_bundle.coin_spends)?,
            spend_bundle.aggregated_signature,
        )
        .await?;

        self.subscribe_coins(subscriptions).await?;

        Ok(())
    }
}
