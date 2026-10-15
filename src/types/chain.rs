use crate::common::errors::EngineError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainFamily {
    Evm,
    Btc,
    Utxo,
    Solana,
    Xrp,
    Cosmos,
    Substrate,
    Algorand,
    Near,
    Aptos,
    Sui,
    Tezos,
    Tron,
    Starknet,
    Hedera,
    Cardano,
    Bep2,
    Other,
}

impl ChainFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Evm => "evm", Self::Btc => "btc", Self::Utxo => "utxo", Self::Solana => "solana",
            Self::Xrp => "xrp", Self::Cosmos => "cosmos", Self::Substrate => "substrate",
            Self::Algorand => "algorand", Self::Near => "near", Self::Aptos => "aptos",
            Self::Sui => "sui", Self::Tezos => "tezos", Self::Tron => "tron",
            Self::Starknet => "starknet", Self::Hedera => "hedera", Self::Cardano => "cardano",
            Self::Bep2 => "bep2", Self::Other => "other",
        }
    }
}

impl From<&str> for ChainFamily {
    fn from(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "evm" | "ethereum" | "polygon" | "base" | "arbitrum" | "optimism" | "avalanche" | "bnb" | "bsc" => Self::Evm,
            "btc" | "bitcoin" | "bitcoin-cash" | "bch" | "litecoin" | "ltc" | "doge" | "dogecoin" => Self::Btc,
            "utxo" => Self::Utxo,
            "solana" => Self::Solana,
            "xrp" => Self::Xrp,
            "cosmos" | "atom" | "osmosis" | "osmo" | "secret" | "scrt" | "injective" | "inj" | "celestia" | "tia" => Self::Cosmos,
            "polkadot" | "dot" | "kusama" | "ksm" | "substrate" | "westend" | "asset-hub" => Self::Substrate,
            "algorand" | "algo" => Self::Algorand,
            "near" => Self::Near,
            "aptos" | "apt" => Self::Aptos,
            "sui" => Self::Sui,
            "tezos" | "xtz" => Self::Tezos,
            "tron" | "trx" => Self::Tron,
            "starknet" | "strk" => Self::Starknet,
            "hedera" | "hbar" => Self::Hedera,
            "cardano" | "ada" => Self::Cardano,
            "bep2" => Self::Bep2,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainRequest {
    pub name: String,
    pub chain_id: String,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(rename = "chain_type")]
    pub chain_type: String,
    #[serde(default)]
    pub derivation_path: Option<String>,
    /// UTXO address policy: p2pkh, p2wpkh, or p2sh-p2wpkh.
    /// Other adapter families ignore this field.
    #[serde(default)]
    pub address_type: Option<String>,
    /// UTXO legacy P2PKH Base58Check version byte.
    #[serde(default)]
    pub p2pkh_version: Option<u8>,
    /// UTXO legacy P2SH Base58Check version byte.
    #[serde(default)]
    pub p2sh_version: Option<u8>,
    /// UTXO native SegWit Bech32 human-readable prefix.
    #[serde(default)]
    pub bech32_hrp: Option<String>,
    #[serde(default)]
    pub rpc_provider: Option<String>,
    #[serde(default)]
    pub rpc_url: Option<String>,
}

impl ChainRequest {
    pub fn new(name: impl Into<String>, chain_type: impl Into<String>) -> Self {
        Self { name: name.into(), chain_id: String::new(), symbol: None, network: None, chain_type: chain_type.into(), derivation_path: None, rpc_provider: None, rpc_url: None, address_type: None, p2pkh_version: None, p2sh_version: None, bech32_hrp: None }
    }

    pub fn family(&self) -> ChainFamily { ChainFamily::from(self.chain_type.as_str()) }

    pub fn normalized_type(&self) -> String {
        self.family().as_str().to_string()
    }

    pub fn path(&self) -> Result<String, EngineError> {
        if let Some(path) = self.derivation_path.as_deref() {
            if path.trim().is_empty() {
                return Err(EngineError::Validation("derivation_path cannot be empty".into()));
            }
            return Ok(path.to_string());
        }
        crate::adapters::registry::default_derivation_path(&self.chain_type)
            .ok_or_else(|| EngineError::Unsupported(format!("no default derivation path for chain_type {}", self.chain_type)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainInfo {
    pub name: String,
    pub chain_id: String,
    pub chain_type: String,
    pub derivation_path: String,
}

impl ChainInfo {
    pub fn from_request(request: &ChainRequest) -> Result<Self, EngineError> {
        if request.chain_id.trim().is_empty() { return Err(EngineError::Validation(format!("chain_id is required for blockchain {}", request.name))); }
        Ok(Self { name: request.name.clone(), chain_id: request.chain_id.clone(), chain_type: request.normalized_type(), derivation_path: request.path()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_request_is_metadata_not_a_blockchain_implementation() {
        let c = ChainRequest::new("Ethereum", "EVM");
        assert_eq!(c.family(), ChainFamily::Evm);
        assert_eq!(c.normalized_type(), "evm");
        assert_eq!(c.path().unwrap(), "m/44'/60'/0'/0/0");
    }
}
