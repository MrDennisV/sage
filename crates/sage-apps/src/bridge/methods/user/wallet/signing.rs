use async_trait::async_trait;
use sage_api::SignCoinSpends;
use sage_api::wallet_connect::SignMessageWithPublicKey;

use crate::{
    BridgeApprovalRequestResult, BridgeContext, BridgeHandleResult, BridgeMethod,
    BridgeMethodCapability, BridgeMethodHandleError, BridgeTools, RustBridgeApprovalBody,
    RustBridgeApprovalRequest, RustBridgeRequest, UserBridgeCapability, parse_required_params,
};

#[derive(Debug, Clone, Copy)]
pub struct WalletSignCoinSpends;

#[async_trait]
impl BridgeMethod for WalletSignCoinSpends {
    fn name(&self) -> &'static str {
        "wallet.signCoinSpends"
    }

    fn capability(&self) -> BridgeMethodCapability {
        BridgeMethodCapability::user(UserBridgeCapability::WalletSignCoinSpends)
    }

    fn approval_request(
        &self,
        _ctx: BridgeContext<'_>,
        request: &RustBridgeRequest,
    ) -> BridgeApprovalRequestResult {
        let params = parse_required_params::<SignCoinSpends>(self, request)?;
        Ok(Some(RustBridgeApprovalRequest {
            body: RustBridgeApprovalBody::SignCoinSpends {
                coin_spends: params.coin_spends,
            },
        }))
    }

    async fn handle(
        &self,
        _ctx: BridgeContext<'_>,
        tools: BridgeTools<'_>,
        request: &RustBridgeRequest,
    ) -> BridgeHandleResult {
        let params: SignCoinSpends = parse_required_params(self, request)?;

        let sage = tools.app_state.lock().await;

        let result = sage.sign_coin_spends(params).await.map_err(|err| {
            BridgeMethodHandleError::internal_error(format!("{} failed: {err}", self.name()))
        })?;

        Ok(Box::new(result))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WalletSignMessage;

#[async_trait]
impl BridgeMethod for WalletSignMessage {
    fn name(&self) -> &'static str {
        "wallet.signMessage"
    }

    fn capability(&self) -> BridgeMethodCapability {
        BridgeMethodCapability::user(UserBridgeCapability::WalletSignMessage)
    }

    fn approval_request(
        &self,
        _ctx: BridgeContext<'_>,
        request: &RustBridgeRequest,
    ) -> BridgeApprovalRequestResult {
        let params = parse_required_params::<SignMessageWithPublicKey>(self, request)?;
        Ok(Some(RustBridgeApprovalRequest {
            body: RustBridgeApprovalBody::SignMessage {
                message: params.message,
            },
        }))
    }

    async fn handle(
        &self,
        _ctx: BridgeContext<'_>,
        tools: BridgeTools<'_>,
        request: &RustBridgeRequest,
    ) -> BridgeHandleResult {
        let params: SignMessageWithPublicKey = parse_required_params(self, request)?;

        let sage = tools.app_state.lock().await;

        let result = sage
            .sign_message_with_public_key(params)
            .await
            .map_err(|err| {
                BridgeMethodHandleError::internal_error(format!("{} failed: {err}", self.name()))
            })?;

        Ok(Box::new(result))
    }
}
