use k256::elliptic_curve::sec1::ToEncodedPoint;
use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key, derive_master_key};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_secp256k1_payload;
use crate::crypto::chain_addresses::cosmos_secp256k1_address;
use reqwest::Client;
use serde::Deserialize;

/// Minimal chain configuration for the BEP2 adapter.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub name: Option<String>,
}

/// Asset representation used by the adapter.
#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

pub struct Bep2Adapter {
    client: Option<Client>,
    api_url: String,
}

impl Bep2Adapter {
    pub fn new(chain: ChainConfig) -> Self {
        let api_url = chain.rpc_url.clone();
        let client = if api_url.is_empty() { None } else { Some(Client::new()) };
        Self { client, api_url }
    }


}

#[derive(Deserialize)]
struct Bep2Balance {
    symbol: String,
    free: String,
}

impl WalletAdapter for Bep2Adapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
            cosmos_secp256k1_address(public_key, "bnb")
        }
        fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
            let master = derive_master_key(seed)?;
            let parsed = DerivationPath::parse(path.to_string())?;
            let child = derive_child_key(&master, &parsed)?;
            let secret = k256::SecretKey::from_bytes((&child.key).into()).map_err(|_| EngineError::Crypto("invalid secp256k1 key".into()))?;
            let public = secret.public_key().to_encoded_point(true);
            cosmos_secp256k1_address(public.as_bytes(), "bnb")
        }
        fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
            sign_secp256k1_payload(private_key, tx_data)
        }
}

#[cfg(test)]
mod tests {
    use super::*;

}
