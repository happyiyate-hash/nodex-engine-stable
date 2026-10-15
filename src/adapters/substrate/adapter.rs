use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::signing::sign_sr25519_payload;
use schnorrkel::{Keypair, MiniSecretKey, ExpansionMode};
use blake2::{Blake2b512, Digest as BlakeDigest};

/// Common chain configuration used by Polkadot/Kusama-style adapters.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub name: Option<String>,
    pub ss58_prefix: u16,
    pub native_symbol: String,
    pub native_decimals: u32,
}

impl ChainConfig {
    pub fn polkadot(rpc_url: String) -> Self {
        Self { rpc_url, name: Some("polkadot".into()), ss58_prefix: 0, native_symbol: "DOT".into(), native_decimals: 10 }
    }

    pub fn kusama(rpc_url: String) -> Self {
        Self { rpc_url, name: Some("kusama".into()), ss58_prefix: 2, native_symbol: "KSM".into(), native_decimals: 12 }
    }
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

pub struct SubstrateAdapter {
    api_url: String,
    ss58_prefix: u16,
    native_symbol: String,
    native_decimals: u32,
}

impl SubstrateAdapter {
    pub fn config_for_network(network: &str, rpc_url: impl Into<String>) -> Result<ChainConfig, EngineError> {
        let rpc_url = rpc_url.into();
        match network.trim().to_ascii_lowercase().as_str() {
            "polkadot" | "dot" => Ok(ChainConfig::polkadot(rpc_url)),
            "kusama" | "ksm" => Ok(ChainConfig::kusama(rpc_url)),
            "westend" => {
                let mut config = ChainConfig::polkadot(rpc_url);
                config.name = Some("westend".into());
                config.native_symbol = "WND".into();
                Ok(config)
            }
            "asset-hub" | "assethub" | "polkadot-asset-hub" => {
                let mut config = ChainConfig::polkadot(rpc_url);
                config.name = Some("asset-hub".into());
                Ok(config)
            }
            other => Err(EngineError::Unsupported(format!("unsupported Substrate network: {other}"))),
        }
    }

    pub fn address_for_chain(public_key: &[u8], ss58_prefix: u16) -> Result<String, EngineError> {
        if public_key.len() != 32 { return Err(EngineError::Validation("Substrate public key must be 32 bytes".into())); }
        let mut data = Vec::with_capacity(35);
        data.push(ss58_prefix as u8);
        data.extend_from_slice(public_key);
        let mut preimage = b"SS58PRE".to_vec();
        preimage.extend_from_slice(&data);
        let digest = Blake2b512::digest(&preimage);
        data.extend_from_slice(&digest[..2]);
        Ok(bs58::encode(data).into_string())
    }

    pub fn new(chain: ChainConfig) -> Self {
        Self {
            api_url: chain.rpc_url.clone(),
            ss58_prefix: chain.ss58_prefix,
            native_symbol: chain.native_symbol,
            native_decimals: chain.native_decimals,
        }
    }

    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter().map(|mut asset| {
            if let Some(decimals) = asset.decimals {
                asset.balance = normalize_base_amount(&asset.balance, decimals)?;
            }
            Ok(asset)
        }).collect()
    }



}

fn normalize_base_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
    let value = raw.trim().parse::<u128>()
        .map_err(|_| EngineError::Validation("invalid Polkadot base amount".into()))?;
    if decimals == 0 { return Ok(value.to_string()); }
    let scale = 10u128.checked_pow(decimals)
        .ok_or_else(|| EngineError::Validation("Substrate decimal exponent is too large".into()))?;
    let whole = value / scale;
    let fraction = value % scale;
    if fraction == 0 { return Ok(whole.to_string()); }
    let mut fraction_text = format!("{:0width$}", fraction, width = decimals as usize);
    while fraction_text.ends_with('0') { fraction_text.pop(); }
    Ok(format!("{}.{}", whole, fraction_text))
}

impl WalletAdapter for SubstrateAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 32 {
            return Err(EngineError::Validation("Substrate public key must be 32 bytes".into()));
        }
        let mut data = vec![0u8];
        data.extend_from_slice(public_key);
        let mut preimage = b"SS58PRE".to_vec();
        preimage.extend_from_slice(&data);
        let digest = Blake2b512::digest(&preimage);
        data.extend_from_slice(&digest[..2]);
        Ok(bs58::encode(data).into_string())
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        if path.trim().is_empty() {
            return Err(EngineError::Validation(
                "Substrate derivation_path is required".into(),
            ));
        }
        if seed.len() != 32 {
            return Err(EngineError::Validation(
                "Substrate seed must be 32 bytes".into(),
            ));
        }

        // Validate the path here. Substrate path interpretation is intentionally
        // adapter-owned; the generic wallet flow only passes the caller's path.
        let _ = crate::crypto::derivation::DerivationPath::parse(path.to_string())?;

        let master = MiniSecretKey::from_bytes(seed)
            .map_err(|_| EngineError::Validation("Substrate seed must be 32 bytes".into()))?;
        let keypair: Keypair = master.expand_to_keypair(ExpansionMode::Ed25519);
        Self::pub_key_to_address(keypair.public.to_bytes().as_ref())
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        sign_sr25519_payload(private_key, tx_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_planck_exactly() {
        let assets = vec![AssetRow {
            address: String::new(),
            symbol: "DOT".into(),
            decimals: Some(10),
            is_native: true,
            balance: "12345678901".into(),
        }];
        let normalized = SubstrateAdapter::normalize_balances(assets).unwrap();
        assert_eq!(normalized[0].balance, "1.2345678901");
    }

    #[test]
    fn rejects_invalid_planck() {
        let assets = vec![AssetRow {
            address: String::new(),
            symbol: "DOT".into(),
            decimals: Some(10),
            is_native: true,
            balance: "invalid".into(),
        }];
        assert!(SubstrateAdapter::normalize_balances(assets).is_err());
    }
}
