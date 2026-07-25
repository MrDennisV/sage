#[cfg(feature = "native")]
use std::time::Duration;

#[cfg(feature = "native")]
use itertools::Itertools;
#[cfg(feature = "native")]
use sage_api::{
    AddPeer, AddPeerResponse, GetPeers, GetPeersResponse, PeerRecord, RemovePeer,
    RemovePeerResponse, SetChangeAddressResponse, SetDiscoverPeers, SetDiscoverPeersResponse,
    SetNetwork, SetNetworkOverride, SetNetworkOverrideResponse, SetNetworkResponse, SetTargetPeers,
    SetTargetPeersResponse,
};
use sage_api::{
    GetNetwork, GetNetworkResponse, GetNetworks, GetNetworksResponse, SetChangeAddress,
    SetDeltaSync, SetDeltaSyncOverride, SetDeltaSyncOverrideResponse, SetDeltaSyncResponse,
    SetNetworkApiUrl, SetNetworkApiUrlResponse,
};
use sage_database::SqlExecutor;
#[cfg(feature = "native")]
use sage_wallet::SyncCommand;

use crate::{Error, Result, Sage};

impl<E: SqlExecutor> Sage<E> {
    pub fn get_networks(&mut self, _req: GetNetworks) -> Result<GetNetworksResponse> {
        Ok(self.network_list.clone())
    }

    pub fn set_network_api_url(
        &mut self,
        req: SetNetworkApiUrl,
    ) -> Result<SetNetworkApiUrlResponse> {
        let network = self
            .network_list
            .networks
            .iter_mut()
            .find(|network| network.name == req.name)
            .ok_or(Error::UnknownNetwork)?;

        network.api_url = req.api_url.filter(|url| !url.trim().is_empty());

        self.save_config()?;

        Ok(SetNetworkApiUrlResponse {})
    }

    pub fn get_network(&mut self, _req: GetNetwork) -> Result<GetNetworkResponse> {
        let network = self.network();

        Ok(GetNetworkResponse {
            network: network.clone(),
            kind: self.network_kind(),
        })
    }

    pub fn set_delta_sync(&mut self, req: SetDeltaSync) -> Result<SetDeltaSyncResponse> {
        self.wallet_config.defaults.delta_sync = req.delta_sync;
        self.save_config()?;
        Ok(SetDeltaSyncResponse {})
    }

    pub fn set_delta_sync_override(
        &mut self,
        req: SetDeltaSyncOverride,
    ) -> Result<SetDeltaSyncOverrideResponse> {
        let Some(wallet_config) = self
            .wallet_config
            .wallets
            .iter_mut()
            .find(|w| w.fingerprint == req.fingerprint)
        else {
            return Err(Error::UnknownFingerprint);
        };
        wallet_config.delta_sync = req.delta_sync;
        self.save_config()?;
        Ok(SetDeltaSyncOverrideResponse {})
    }

    /// Records the change address for a wallet. The address is baked into the
    /// wallet when it is built, so callers rebuild the wallet afterwards for
    /// the new address to take effect.
    pub fn set_change_address_config(&mut self, req: SetChangeAddress) -> Result<()> {
        let Some(wallet_config) = self
            .wallet_config
            .wallets
            .iter_mut()
            .find(|w| w.fingerprint == req.fingerprint)
        else {
            return Err(Error::UnknownFingerprint);
        };

        wallet_config.change_address = req.change_address;
        self.save_config()
    }
}

// Peer management and anything that reopens the wallet database belongs to the
// native sync manager.
#[cfg(feature = "native")]
impl Sage {
    pub async fn get_peers(&self, _req: GetPeers) -> Result<GetPeersResponse> {
        let peer_state = self.peer_state.lock().await;

        Ok(GetPeersResponse {
            peers: peer_state
                .peers_with_heights()
                .into_iter()
                .sorted_by_key(|info| info.0.socket_addr().ip())
                .map(|info| {
                    let ip = info.0.socket_addr().ip();
                    PeerRecord {
                        ip_addr: ip.to_string(),
                        port: info.0.socket_addr().port(),
                        peak_height: info.1,
                        user_managed: peer_state.peer(ip).is_some_and(|p| p.user_managed),
                    }
                })
                .collect(),
        })
    }

    pub async fn remove_peer(&self, req: RemovePeer) -> Result<RemovePeerResponse> {
        let mut peer_state = self.peer_state.lock().await;

        let ip = req.ip.parse()?;

        if req.ban {
            peer_state.ban(ip, Duration::from_secs(60 * 60), "manually banned");
        } else {
            peer_state.remove_peer(ip);
        }

        Ok(RemovePeerResponse {})
    }

    pub async fn add_peer(&self, req: AddPeer) -> Result<AddPeerResponse> {
        self.command_sender
            .send(SyncCommand::ConnectPeer {
                ip: req.ip.parse()?,
                user_managed: true,
            })
            .await?;

        Ok(AddPeerResponse {})
    }

    pub async fn set_discover_peers(
        &mut self,
        req: SetDiscoverPeers,
    ) -> Result<SetDiscoverPeersResponse> {
        if self.config.network.discover_peers != req.discover_peers {
            self.config.network.discover_peers = req.discover_peers;
            self.save_config()?;
            self.command_sender
                .send(SyncCommand::SetDiscoverPeers(req.discover_peers))
                .await?;
        }

        Ok(SetDiscoverPeersResponse {})
    }

    pub async fn set_target_peers(
        &mut self,
        req: SetTargetPeers,
    ) -> Result<SetTargetPeersResponse> {
        self.config.network.target_peers = req.target_peers;
        self.save_config()?;
        self.command_sender
            .send(SyncCommand::SetTargetPeers(req.target_peers as usize))
            .await?;

        Ok(SetTargetPeersResponse {})
    }

    pub async fn set_network(&mut self, req: SetNetwork) -> Result<SetNetworkResponse> {
        self.select_network(req.name)?;
        self.switch_wallet().await?;
        self.setup_peers().await?;
        Ok(SetNetworkResponse {})
    }

    pub async fn set_network_override(
        &mut self,
        req: SetNetworkOverride,
    ) -> Result<SetNetworkOverrideResponse> {
        self.override_wallet_network(req.fingerprint, req.name)?;
        self.switch_wallet().await?;
        self.setup_peers().await?;

        Ok(SetNetworkOverrideResponse {})
    }

    pub async fn set_change_address(
        &mut self,
        req: SetChangeAddress,
    ) -> Result<SetChangeAddressResponse> {
        self.set_change_address_config(req)?;
        self.switch_wallet().await?;
        Ok(SetChangeAddressResponse {})
    }
}
