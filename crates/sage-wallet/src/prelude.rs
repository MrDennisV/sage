// Re-export common types for wallet code.
// With `native`: delegates to chia-wallet-sdk's prelude (full SDK).
// Without `native`: sources from individual WASM-compatible sub-crates.

// Types from chia-sdk-driver (spend building, actions, assets)
pub use chia_sdk_driver::{
    Action, Arbitrage, ArbitrageSide, AssetInfo, Cat, CatAssetInfo, CatInfo, CatSpend, ClawbackV2,
    CurriedPuzzle, Delta, Deltas, Did, DidInfo, DriverError, HashedPtr, Id, Launcher, Layer, Nft,
    NftAssetInfo, NftInfo, NftMint, Offer, OfferAmounts, OfferCoins, OptionAssetInfo,
    OptionContract, OptionInfo, OptionLauncher, OptionLauncherInfo, OptionMetadata, OptionType,
    OptionUnderlying, Outputs, Puzzle, RawPuzzle, Relation, RequestedPayments, RoyaltyInfo,
    SettlementLayer, Singleton, SingletonInfo, Spend, SpendAction, SpendContext,
    SpendWithConditions, Spends, StandardLayer, Vault, VaultInfo,
};

// Types from chia-sdk-signer (signing)
pub use chia_sdk_signer::{
    AggSigConstants, RequiredBlsSignature, RequiredSecpSignature, RequiredSignature,
};

// Types from chia-sdk-types (conditions, compilation, CLVM)
pub use chia_sdk_types::{
    Compilation, Condition, Conditions, MAINNET_CONSTANTS, MerkleProof, MerkleTree, Mod,
    TESTNET11_CONSTANTS, compile_chialisp, compile_rue, conditions::*, run_puzzle,
};

// Types from chia-sdk-utils (address, coin selection)
pub use chia_sdk_utils::{Address, Bech32, parse_hex, select_coins};

// Puzzle hashes (constants)
pub use chia_puzzles::{
    NFT_METADATA_UPDATER_DEFAULT_HASH, SETTLEMENT_PAYMENT_HASH, SINGLETON_LAUNCHER_HASH,
};

// Core chia types
pub use chia_bls::{PublicKey, SecretKey, Signature};
pub use chia_protocol::{Bytes, Bytes32, Coin, CoinSpend, CoinState, Program, SpendBundle};
pub use clvm_traits::{self, FromClvm, ToClvm};
pub use clvm_utils::{
    CurriedProgram, ToTreeHash, TreeHash, tree_hash, tree_hash_atom, tree_hash_pair,
};
pub use clvmr::{Allocator, Atom, NodePtr, SExp};

// Native-only: Peer, Simulator, CoinsetClient
#[cfg(feature = "native")]
pub use chia_wallet_sdk::prelude::{Peer, PeerOptions};

#[cfg(feature = "native")]
pub use chia_wallet_sdk::prelude::{
    BlsPair, BlsPairWithCoin, K1Pair, R1Pair, Simulator, SimulatorConfig, SimulatorError,
};
