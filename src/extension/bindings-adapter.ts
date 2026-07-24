// bindings-adapter.ts
// DROP-IN replacement for @/bindings
// Same interface, routes commands through chrome.runtime.sendMessage
// to the service worker backend.

// Re-export ALL types from the original bindings so existing code works
export type {
  Action,
  AddNftUri,
  AddPeer,
  AddressKind,
  Amount,
  Asset,
  AssetCoinType,
  AssetKind,
  AssignNftsToDid,
  AutoCombineCat,
  AutoCombineCatResponse,
  AutoCombineXch,
  AutoCombineXchResponse,
  BulkMintNfts,
  BulkMintNftsResponse,
  BulkSendCat,
  BulkSendXch,
  CancelOffer,
  CancelOffers,
  CheckAddress,
  CheckAddressResponse,
  Coin,
  CoinFilterMode,
  CoinJson,
  CoinRecord,
  CoinSortMode,
  CoinSpend,
  CoinSpendJson,
  Combine,
  CombineOffers,
  CombineOffersResponse,
  CreateDid,
  CreateTransaction,
  DeleteDatabase,
  DeleteDatabaseResponse,
  DeleteKey,
  DeleteKeyResponse,
  DeleteOffer,
  DeleteOfferResponse,
  DeleteUserTheme,
  DeleteUserThemeResponse,
  DerivationRecord,
  DidRecord,
  EmptyResponse,
  Error,
  ErrorKind,
  ExerciseOptions,
  FeeAction,
  FilterUnlockedCoins,
  FilterUnlockedCoinsResponse,
  FinalizeClawback,
  GenerateMnemonic,
  GenerateMnemonicResponse,
  GetAllCats,
  GetAllCatsResponse,
  GetAreCoinsSpendable,
  GetAreCoinsSpendableResponse,
  GetAssetCoins,
  GetCats,
  GetCatsResponse,
  GetCoins,
  GetCoinsByIds,
  GetCoinsByIdsResponse,
  GetCoinsResponse,
  GetDatabaseStats,
  GetDatabaseStatsResponse,
  GetDerivations,
  GetDerivationsResponse,
  GetDids,
  GetDidsResponse,
  GetKey,
  GetKeyResponse,
  GetKeys,
  GetKeysResponse,
  GetMinterDidIds,
  GetMinterDidIdsResponse,
  GetNetwork,
  GetNetworkResponse,
  GetNetworks,
  GetNft,
  GetNftCollection,
  GetNftCollectionResponse,
  GetNftCollections,
  GetNftCollectionsResponse,
  GetNftData,
  GetNftDataResponse,
  GetNftIcon,
  GetNftIconResponse,
  GetNftResponse,
  GetNftThumbnail,
  GetNftThumbnailResponse,
  GetNfts,
  GetNftsResponse,
  GetOffer,
  GetOfferResponse,
  GetOffers,
  GetOffersForAsset,
  GetOffersForAssetResponse,
  GetOffersResponse,
  GetOption,
  GetOptionResponse,
  GetOptions,
  GetOptionsResponse,
  GetPeers,
  GetPeersResponse,
  GetPendingTransactions,
  GetPendingTransactionsResponse,
  GetSecretKey,
  GetSecretKeyResponse,
  GetSpendableCoinCount,
  GetSpendableCoinCountResponse,
  GetSyncStatus,
  GetSyncStatusResponse,
  GetToken,
  GetTokenResponse,
  GetTransaction,
  GetTransactionResponse,
  GetTransactions,
  GetTransactionsResponse,
  GetUserTheme,
  GetUserThemeResponse,
  GetUserThemes,
  GetUserThemesResponse,
  GetVersion,
  GetVersionResponse,
  Id,
  ImportKey,
  ImportKeyResponse,
  ImportOffer,
  ImportOfferResponse,
  IncreaseDerivationIndex,
  IncreaseDerivationIndexResponse,
  InheritedNetwork,
  IsAssetOwned,
  IsAssetOwnedResponse,
  IssueCat,
  KeyInfo,
  KeyKind,
  LineageProof,
  LogFile,
  Login,
  LoginResponse,
  Logout,
  LogoutResponse,
  MakeOffer,
  MakeOfferResponse,
  MintNftAction,
  MintOption,
  MintOptionResponse,
  Network,
  NetworkConfig,
  NetworkKind,
  NetworkList,
  NewNftUri,
  NftCollectionRecord,
  NftData,
  NftMint,
  NftRecord,
  NftRoyalty,
  NftSortMode,
  NftSpecialUseType,
  NftTransfer,
  NftUriKind,
  NormalizeDids,
  OfferAmount,
  OfferAsset,
  OfferRecord,
  OfferRecordStatus,
  OfferSummary,
  OptionAsset,
  OptionAssets,
  OptionRecord,
  OptionSortMode,
  PeerRecord,
  PendingTransactionRecord,
  PerformDatabaseMaintenance,
  PerformDatabaseMaintenanceResponse,
  RedownloadNft,
  RedownloadNftResponse,
  RemovePeer,
  RenameKey,
  RenameKeyResponse,
  Resync,
  ResyncCat,
  ResyncCatResponse,
  ResyncResponse,
  Result,
  SaveUserTheme,
  SaveUserThemeResponse,
  SecretKeyInfo,
  SendAction,
  SendCat,
  SendTransactionImmediately,
  SendTransactionImmediatelyResponse,
  SendXch,
  SetChangeAddress,
  SetDeltaSync,
  SetDeltaSyncOverride,
  SetDiscoverPeers,
  SetNetwork,
  SetNetworkOverride,
  SetTargetPeers,
  SetWalletEmoji,
  SetWalletEmojiResponse,
  SignCoinSpends,
  SignCoinSpendsResponse,
  SignMessageByAddress,
  SignMessageByAddressResponse,
  SignMessageWithPublicKey,
  SignMessageWithPublicKeyResponse,
  SpendBundle,
  SpendBundleJson,
  SpendableCoin,
  Split,
  SubmitTransaction,
  SubmitTransactionResponse,
  SyncEvent,
  TakeOffer,
  TakeOfferResponse,
  TokenRecord,
  TransactionCoinRecord,
  TransactionInput,
  TransactionOutput,
  TransactionRecord,
  TransactionResponse,
  TransactionSummary,
  TransferDids,
  TransferNfts,
  TransferOptions,
  Unit,
  UpdateCat,
  UpdateCatResponse,
  UpdateDid,
  UpdateDidResponse,
  UpdateNft,
  UpdateNftAction,
  UpdateNftCollection,
  UpdateNftCollectionResponse,
  UpdateNftResponse,
  UpdateOption,
  UpdateOptionResponse,
  ViewCoinSpends,
  ViewCoinSpendsResponse,
  ViewOffer,
  ViewOfferResponse,
  Wallet,
  WalletDefaults,
} from '../bindings';

// Helper: send command to service worker via chrome.runtime
async function invoke<T>(cmd: string, args?: Record<string, any>): Promise<T> {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(
      { type: 'COMMAND', cmd, args },
      (response: any) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
          return;
        }
        if (response && response.error) {
          reject(response.error);
          return;
        }
        resolve(response?.data);
      },
    );
  });
}

// Drop-in replacement for Tauri commands — same interface as src/bindings.ts
export const commands = {
  async initialize() {
    return invoke<null>('initialize');
  },
  async login(req: any) {
    return invoke('login', { req });
  },
  async logout(req: any) {
    return invoke('logout', { req });
  },
  async resync(req: any) {
    return invoke('resync', { req });
  },
  async generateMnemonic(req: any) {
    return invoke('generate_mnemonic', { req });
  },
  async importKey(req: any) {
    return invoke('import_key', { req });
  },
  async deleteKey(req: any) {
    return invoke('delete_key', { req });
  },
  async deleteDatabase(req: any) {
    return invoke('delete_database', { req });
  },
  async renameKey(req: any) {
    return invoke('rename_key', { req });
  },
  async getKeys(req: any) {
    return invoke('get_keys', { req });
  },
  async setWalletEmoji(req: any) {
    return invoke('set_wallet_emoji', { req });
  },
  async getKey(req: any) {
    return invoke('get_key', { req });
  },
  async getSecretKey(req: any) {
    return invoke('get_secret_key', { req });
  },
  async sendXch(req: any) {
    return invoke('send_xch', { req });
  },
  async bulkSendXch(req: any) {
    return invoke('bulk_send_xch', { req });
  },
  async combine(req: any) {
    return invoke('combine', { req });
  },
  async split(req: any) {
    return invoke('split', { req });
  },
  async autoCombineXch(req: any) {
    return invoke('auto_combine_xch', { req });
  },
  async sendCat(req: any) {
    return invoke('send_cat', { req });
  },
  async bulkSendCat(req: any) {
    return invoke('bulk_send_cat', { req });
  },
  async autoCombineCat(req: any) {
    return invoke('auto_combine_cat', { req });
  },
  async issueCat(req: any) {
    return invoke('issue_cat', { req });
  },
  async createDid(req: any) {
    return invoke('create_did', { req });
  },
  async bulkMintNfts(req: any) {
    return invoke('bulk_mint_nfts', { req });
  },
  async transferNfts(req: any) {
    return invoke('transfer_nfts', { req });
  },
  async transferDids(req: any) {
    return invoke('transfer_dids', { req });
  },
  async normalizeDids(req: any) {
    return invoke('normalize_dids', { req });
  },
  async mintOption(req: any) {
    return invoke('mint_option', { req });
  },
  async transferOptions(req: any) {
    return invoke('transfer_options', { req });
  },
  async exerciseOptions(req: any) {
    return invoke('exercise_options', { req });
  },
  async addNftUri(req: any) {
    return invoke('add_nft_uri', { req });
  },
  async assignNftsToDid(req: any) {
    return invoke('assign_nfts_to_did', { req });
  },
  async finalizeClawback(req: any) {
    return invoke('finalize_clawback', { req });
  },
  async createTransaction(req: any) {
    return invoke('create_transaction', { req });
  },
  async signCoinSpends(req: any) {
    return invoke('sign_coin_spends', { req });
  },
  async viewCoinSpends(req: any) {
    return invoke('view_coin_spends', { req });
  },
  async submitTransaction(req: any) {
    return invoke('submit_transaction', { req });
  },
  async getSyncStatus(req: any) {
    return invoke('get_sync_status', { req });
  },
  async getVersion(req: any) {
    return invoke('get_version', { req });
  },
  async getDatabaseStats(req: any) {
    return invoke('get_database_stats', { req });
  },
  async performDatabaseMaintenance(req: any) {
    return invoke('perform_database_maintenance', { req });
  },
  async checkAddress(req: any) {
    return invoke('check_address', { req });
  },
  async getDerivations(req: any) {
    return invoke('get_derivations', { req });
  },
  async getAreCoinsSpendable(req: any) {
    return invoke('get_are_coins_spendable', { req });
  },
  async getSpendableCoinCount(req: any) {
    return invoke('get_spendable_coin_count', { req });
  },
  async getCoinsByIds(req: any) {
    return invoke('get_coins_by_ids', { req });
  },
  async getCoins(req: any) {
    return invoke('get_coins', { req });
  },
  async getCats(req: any) {
    return invoke('get_cats', { req });
  },
  async getAllCats(req: any) {
    return invoke('get_all_cats', { req });
  },
  async getToken(req: any) {
    return invoke('get_token', { req });
  },
  async getDids(req: any) {
    return invoke('get_dids', { req });
  },
  async getMinterDidIds(req: any) {
    return invoke('get_minter_did_ids', { req });
  },
  async getOptions(req: any) {
    return invoke('get_options', { req });
  },
  async getOption(req: any) {
    return invoke('get_option', { req });
  },
  async getNftCollections(req: any) {
    return invoke('get_nft_collections', { req });
  },
  async getNftCollection(req: any) {
    return invoke('get_nft_collection', { req });
  },
  async getNfts(req: any) {
    return invoke('get_nfts', { req });
  },
  async getNft(req: any) {
    return invoke('get_nft', { req });
  },
  async getNftData(req: any) {
    return invoke('get_nft_data', { req });
  },
  async getNftIcon(req: any) {
    return invoke('get_nft_icon', { req });
  },
  async getNftThumbnail(req: any) {
    return invoke('get_nft_thumbnail', { req });
  },
  async getPendingTransactions(req: any) {
    return invoke('get_pending_transactions', { req });
  },
  async getTransaction(req: any) {
    return invoke('get_transaction', { req });
  },
  async getTransactions(req: any) {
    return invoke('get_transactions', { req });
  },
  async validateAddress(address: string) {
    return invoke<boolean>('validate_address', { address });
  },
  async makeOffer(req: any) {
    return invoke('make_offer', { req });
  },
  async takeOffer(req: any) {
    return invoke('take_offer', { req });
  },
  async combineOffers(req: any) {
    return invoke('combine_offers', { req });
  },
  async viewOffer(req: any) {
    return invoke('view_offer', { req });
  },
  async importOffer(req: any) {
    return invoke('import_offer', { req });
  },
  async getOffers(req: any) {
    return invoke('get_offers', { req });
  },
  async getOffersForAsset(req: any) {
    return invoke('get_offers_for_asset', { req });
  },
  async getOffer(req: any) {
    return invoke('get_offer', { req });
  },
  async deleteOffer(req: any) {
    return invoke('delete_offer', { req });
  },
  async cancelOffer(req: any) {
    return invoke('cancel_offer', { req });
  },
  async cancelOffers(req: any) {
    return invoke('cancel_offers', { req });
  },
  async networkConfig() {
    return invoke('network_config');
  },
  async setDiscoverPeers(req: any) {
    return invoke('set_discover_peers', { req });
  },
  async setTargetPeers(req: any) {
    return invoke('set_target_peers', { req });
  },
  async setNetwork(req: any) {
    return invoke('set_network', { req });
  },
  async setNetworkOverride(req: any) {
    return invoke('set_network_override', { req });
  },
  async walletConfig(fingerprint: number) {
    return invoke('wallet_config', { fingerprint });
  },
  async defaultWalletConfig() {
    return invoke('default_wallet_config');
  },
  async getNetworks(req: any) {
    return invoke('get_networks', { req });
  },
  async getNetwork(req: any) {
    return invoke('get_network', { req });
  },
  async setDeltaSync(req: any) {
    return invoke('set_delta_sync', { req });
  },
  async setDeltaSyncOverride(req: any) {
    return invoke('set_delta_sync_override', { req });
  },
  async setChangeAddress(req: any) {
    return invoke('set_change_address', { req });
  },
  async updateCat(req: any) {
    return invoke('update_cat', { req });
  },
  async resyncCat(req: any) {
    return invoke('resync_cat', { req });
  },
  async updateDid(req: any) {
    return invoke('update_did', { req });
  },
  async updateOption(req: any) {
    return invoke('update_option', { req });
  },
  async updateNft(req: any) {
    return invoke('update_nft', { req });
  },
  async updateNftCollection(req: any) {
    return invoke('update_nft_collection', { req });
  },
  async redownloadNft(req: any) {
    return invoke('redownload_nft', { req });
  },
  async increaseDerivationIndex(req: any) {
    return invoke('increase_derivation_index', { req });
  },
  async getPeers(req: any) {
    return invoke('get_peers', { req });
  },
  async getUserTheme(req: any) {
    return invoke('get_user_theme', { req });
  },
  async getUserThemes(req: any) {
    return invoke('get_user_themes', { req });
  },
  async saveUserTheme(req: any) {
    return invoke('save_user_theme', { req });
  },
  async deleteUserTheme(req: any) {
    return invoke('delete_user_theme', { req });
  },
  async addPeer(req: any) {
    return invoke('add_peer', { req });
  },
  async removePeer(req: any) {
    return invoke('remove_peer', { req });
  },
  async filterUnlockedCoins(req: any) {
    return invoke('filter_unlocked_coins', { req });
  },
  async getAssetCoins(req: any) {
    return invoke('get_asset_coins', { req });
  },
  async signMessageWithPublicKey(req: any) {
    return invoke('sign_message_with_public_key', { req });
  },
  async signMessageByAddress(req: any) {
    return invoke('sign_message_by_address', { req });
  },
  async sendTransactionImmediately(req: any) {
    return invoke('send_transaction_immediately', { req });
  },
  async isRpcRunning() {
    // RPC server not applicable in extension
    return false;
  },
  async startRpcServer() {
    return null;
  },
  async stopRpcServer() {
    return null;
  },
  async getRpcRunOnStartup() {
    return false;
  },
  async setRpcRunOnStartup(_runOnStartup: boolean) {
    return null;
  },
  async switchWallet() {
    return invoke<null>('switch_wallet');
  },
  async moveKey(fingerprint: number, index: number) {
    return invoke<null>('move_key', { fingerprint, index });
  },
  async downloadCniOffercode(code: string) {
    return invoke<string>('download_cni_offercode', { code });
  },
  async getLogs() {
    return invoke('get_logs');
  },
  async isAssetOwned(req: any) {
    return invoke('is_asset_owned', { req });
  },
};

// Events adapter — Sage uses Tauri events for sync notifications
type EventCallback<T> = (event: { payload: T }) => void;

interface EventObj<T> {
  listen: (cb: EventCallback<T>) => Promise<() => void>;
  once: (cb: EventCallback<T>) => Promise<() => void>;
  emit: (payload?: T) => Promise<void>;
}

export const events = {
  syncEvent: {
    listen: (handler: EventCallback<any>): Promise<() => void> => {
      const listener = (message: any) => {
        if (message && message.type === 'SYNC_EVENT') {
          handler({ payload: message.data });
        }
      };
      chrome.runtime.onMessage.addListener(listener);
      return Promise.resolve(() => {
        chrome.runtime.onMessage.removeListener(listener);
      });
    },
    once: (handler: EventCallback<any>): Promise<() => void> => {
      const listener = (message: any) => {
        if (message && message.type === 'SYNC_EVENT') {
          handler({ payload: message.data });
          chrome.runtime.onMessage.removeListener(listener);
        }
      };
      chrome.runtime.onMessage.addListener(listener);
      return Promise.resolve(() => {
        chrome.runtime.onMessage.removeListener(listener);
      });
    },
    emit: async (_payload?: any): Promise<void> => {},
  } as EventObj<any>,
};
