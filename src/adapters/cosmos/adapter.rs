#[path = "adapter/derivation.rs"]
mod derivation;
#[path = "adapter/signing.rs"]
mod signing;
#[path = "adapter/balances.rs"]
mod balances;

use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::chain_addresses::cosmos_secp256k1_address;
use crate::types::chain::ChainRequest;

/// Runtime configuration for a Cosmos SDK-compatible chain.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub name: Option<String>,
    pub chain_type: String,
}

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

/// Coordinates Cosmos-family operations; chain-specific mechanics live in
/// derivation.rs, signing.rs, and balances.rs.
pub struct CosmosAdapter;

impl CosmosAdapter {
    /// Kept for compatibility with older callers. New request paths should
    /// pass ChainRequest.bech32_hrp to derive_addresses.
    pub fn prefix_for_chain(chain_type: &str) -> &'static str {
        match chain_type.trim().to_ascii_lowercase().as_str() {
            "osmosis" | "osmo" => "osmo",
            "secret" | "scrt" => "secret",
            "injective" | "inj" | "nativeinjective" => "inj",
            "celestia" | "tia" => "celestia",
            "agoric" => "agoric",
            "akash" => "akash",
            "axelar" => "axelar",
            "band" | "bandchain" => "band",
            "comdex" => "comdex",
            "coreum" => "core",
            "cosmos" | "cosmoshub" | "cosmos-hub" => "cosmos",
            "crescent" => "cre",
            "cryptoorg" | "crypto.org" | "cro" => "cro",
            "fetch" | "fetchai" => "fetch",
            "iris" | "irisnet" => "iaa",
            "juno" => "juno",
            "kava" => "kava",
            "kujira" => "kujira",
            "mars" | "mars-hub" => "mars",
            "canto" | "nativecanto" => "canto",
            "neutron" => "neutron",
            "noble" => "noble",
            "persistence" => "persistence",
            "quasar" => "quasar",
            "sei" => "sei",
            "sommelier" => "somm",
            "stargaze" => "stars",
            "stride" => "stride",
            "terra" | "terraclassic" | "terra-classic" => "terra",
            "umee" => "umee",
            _ => "cosmos",
        }
    }

    pub fn derive_addresses(
        seed: &[u8],
        chains: &[ChainRequest],
    ) -> Result<std::collections::HashMap<String, String>, EngineError> {
        derivation::derive_addresses(seed, chains)
    }

    pub fn derive_from_mnemonic(
        mnemonic: &str,
        derivation_path: &str,
        chain_type: &str,
    ) -> Result<String, EngineError> {
        let seed = crate::crypto::mnemonic::mnemonic_to_seed(mnemonic, "")?;
        let hrp = derivation::prefix_for_chain(chain_type, None)?;
        derivation::derive_address(&seed, derivation_path, &hrp)
    }

    pub fn derive_address_for_chain(
        seed: &[u8],
        path: &str,
        chain_type: &str,
    ) -> Result<String, EngineError> {
        let hrp = derivation::prefix_for_chain(chain_type, None)?;
        derivation::derive_address(seed, path, &hrp)
    }

    pub fn pub_key_to_address_for_chain(
        public_key: &[u8],
        chain_type: &str,
    ) -> Result<String, EngineError> {
        let hrp = derivation::prefix_for_chain(chain_type, None)?;
        cosmos_secp256k1_address(public_key, &hrp)
    }

    pub fn normalize_balances_for_chain(
        _chain_type: &str,
        assets: Vec<AssetRow>,
    ) -> Result<Vec<AssetRow>, EngineError> {
        balances::normalize_assets(assets)
    }

    pub fn normalize_balances(assets: Vec<AssetRow>) -> Vec<AssetRow> {
        balances::normalize_assets_lossy(assets)
    }
}

impl WalletAdapter for CosmosAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        Self::pub_key_to_address_for_chain(public_key, "cosmos")
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        Self::derive_address_for_chain(seed, path, "cosmos")
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        signing::sign_payload(private_key, tx_data)
    }
}
