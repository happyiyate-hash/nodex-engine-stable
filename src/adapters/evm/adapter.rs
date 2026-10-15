use crate::common::errors::EngineError;
use crate::adapters::adapter_trait::WalletAdapter;
use crate::crypto::bip32::{derive_child_key, derive_master_key};
use crate::crypto::derivation::DerivationPath;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{Signer, Wallet};
use ethers::types::{Address, Bytes, TransactionRequest, U256};
use ethers::contract::Contract;
use ethers::types::transaction::eip2718::TypedTransaction;
use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};
use std::sync::Arc;

/// Minimal chain configuration used by the adapter factory.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_urls: Vec<String>,
}

/// Asset representation for balance queries.
#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

/// EVM family adapter. Network-specific settings are supplied through ChainConfig.
pub struct EvmAdapter {
    provider: Option<Arc<Provider<Http>>>,
    chain: ChainConfig,
}

impl EvmAdapter {
    /// Canonical derivation policy for wallets created by this engine.
    pub const CANONICAL_DERIVATION_PATH: &'static str = "m/44'/60'/0'/0/0";

    pub fn derive_canonical_address(seed: &[u8]) -> Result<String, EngineError> {
        Self::derive_address_from_path(seed, Self::CANONICAL_DERIVATION_PATH)
    }

    pub fn new(chain: ChainConfig, infura_key: Option<&str>, alchemy_key: Option<&str>) -> Self {
        let rpc_url = chain.rpc_urls.first().cloned().or_else(|| {
            alchemy_key.map(|k| format!("https://eth-mainnet.g.alchemy.com/v2/{k}"))
        }).or_else(|| {
            infura_key.map(|k| format!("https://mainnet.infura.io/v3/{k}"))
        });

        let provider = rpc_url.and_then(|url| Provider::<Http>::try_from(url).ok()).map(Arc::new);
        Self { provider, chain }
    }

    /// Returns basic on-chain account information. Security analytics such as
    /// approvals require additional token enumeration and are intentionally not
    /// represented as fake values here.
    pub async fn check_account_security(&self, address: &str) -> Option<serde_json::Value> {
        let provider = self.provider.as_ref()?;
        let addr = address.parse::<Address>().ok()?;
        let code = provider.get_code(addr, None).await.ok()?;
        let balance = provider.get_balance(addr, None).await.ok()?;
        let is_contract = !code.0.is_empty();
        let gas_status = if balance > U256::exp10(16) { "Optimal" } else { "Low Gas Warning" };

        Some(serde_json::json!({
            "walletType": if is_contract { "Smart Contract Wallet" } else { "EOA" },
            "isContract": is_contract,
            "gasStatus": gas_status,
            "activeApprovals": null,
            "unlimitedApprovals": null,
            "securityScore": null
        }))
    }

    /// Normalize raw EVM balances returned by a Balance Provider.
    /// Network access stays in the provider/RPC layer; this method only applies
    /// EVM-specific decimal normalization.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Vec<AssetRow> {
        assets.into_iter().map(|mut asset| {
            let decimals = asset.decimals.unwrap_or(18);
            asset.balance = normalize_raw_balance(&asset.balance, decimals)
                .unwrap_or_else(|| "0".to_string());
            asset
        }).collect()
    }

    /// Backward-compatible balance entry point. New Balance Service code should
    /// fetch through a Balance Provider and then call normalize_balances().


    /// Sign an EVM transaction encoded as JSON. Supported fields are:
    /// `type` (0 or 2), `chain_id`, `nonce`, `to`, `value`, `data`, `gas`,
    /// `gas_price`, `max_fee_per_gas`, and `max_priority_fee_per_gas`.
    /// Returns the fully serialized raw signed transaction.
    pub async fn sign_transaction_json(&self, private_key: &[u8], tx_json: &[u8]) -> Result<Vec<u8>, EngineError> {
        sign_transaction_json_with_chain(private_key, tx_json, self.chain.chain_id).await
    }

    pub fn sign_transaction_for_chain(private_key: &[u8], tx_data: &[u8], chain_id: u64) -> Result<Vec<u8>, EngineError> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()
            .map_err(|e| EngineError::Unexpected(format!("failed to initialize signing runtime: {e}")))?;
        rt.block_on(sign_transaction_json_with_chain(private_key, tx_data, chain_id))
    }

    pub fn derive_private_key(seed: &[u8], path: &str) -> Result<[u8; 32], EngineError> {
        let derivation_path = DerivationPath::parse(path.to_string())?;
        let master_key = derive_master_key(seed)?;
        let child_key = derive_child_key(&master_key, &derivation_path)?;
        Ok(child_key.key)
    }
}

async fn sign_transaction_json_with_chain(
    private_key: &[u8],
    tx_json: &[u8],
    default_chain_id: u64,
) -> Result<Vec<u8>, EngineError> {
    let key: [u8; 32] = private_key.try_into()
        .map_err(|_| EngineError::Validation("EVM private key must be exactly 32 bytes".into()))?;
    let signing_key = SigningKey::from_bytes((&key).into())
        .map_err(|_| EngineError::Crypto("invalid secp256k1 private key".into()))?;
    let wallet: Wallet<SigningKey> = Wallet::from(signing_key);
    let value: serde_json::Value = serde_json::from_slice(tx_json)
        .map_err(|e| EngineError::Validation(format!("invalid EVM transaction JSON: {e}")))?;

    let chain_id = value.get("chain_id").and_then(|v| v.as_u64()).unwrap_or(default_chain_id);
    let nonce = parse_u256_field(&value, "nonce")?;
    let gas = parse_u256_field(&value, "gas")?;
    let to = parse_optional_address(&value, "to")?;
    let tx_value = parse_u256_field(&value, "value")?.unwrap_or_default();
    let data = parse_bytes_field(&value, "data")?.unwrap_or_default();
    let tx_type = value.get("type").and_then(|v| v.as_u64()).unwrap_or(0);

    let mut tx = if tx_type == 2 || value.get("max_fee_per_gas").is_some() || value.get("max_priority_fee_per_gas").is_some() {
        let max_fee = parse_required_u256_field(&value, "max_fee_per_gas")?;
        let priority = parse_required_u256_field(&value, "max_priority_fee_per_gas")?;
        TypedTransaction::Eip1559(ethers::types::transaction::eip1559::Eip1559TransactionRequest {
            from: None,
            to: to.map(Into::into),
            value: Some(tx_value),
            data: Some(data),
            nonce: Some(nonce.unwrap_or_default()),
            access_list: Default::default(),
            max_priority_fee_per_gas: Some(priority),
            max_fee_per_gas: Some(max_fee),
            gas: Some(gas.unwrap_or_default()),
            chain_id: Some(chain_id.into()),
        })
    } else {
        TypedTransaction::Legacy(TransactionRequest {
            from: None,
            to: to.map(Into::into),
            gas: Some(gas.unwrap_or_default()),
            gas_price: Some(parse_required_u256_field(&value, "gas_price")?),
            value: Some(tx_value),
            data: Some(data),
            nonce: Some(nonce.unwrap_or_default()),
            chain_id: Some(chain_id.into()),
        })
    };

    wallet.sign_transaction(&tx).await
        .map(|sig| tx.rlp_signed(&sig).to_vec())
        .map_err(|e| EngineError::Crypto(format!("EVM transaction signing failed: {e}")))
}

fn parse_u256_field(value: &serde_json::Value, name: &str) -> Result<Option<U256>, EngineError> {
    match value.get(name) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => parse_u256_value(v).map(Some),
    }
}

fn parse_required_u256_field(value: &serde_json::Value, name: &str) -> Result<U256, EngineError> {
    parse_u256_field(value, name)?.ok_or_else(|| EngineError::Validation(format!("missing required EVM transaction field: {name}")))
}

fn parse_u256_value(value: &serde_json::Value) -> Result<U256, EngineError> {
    if let Some(n) = value.as_u64() { return Ok(n.into()); }
    if let Some(s) = value.as_str() {
        if let Some(hex) = s.strip_prefix("0x") {
            return U256::from_str_radix(hex, 16).map_err(|_| EngineError::Validation("invalid hexadecimal integer".into()));
        }
        return U256::from_dec_str(s).map_err(|_| EngineError::Validation("invalid decimal integer".into()));
    }
    Err(EngineError::Validation("EVM integer must be a number or string".into()))
}

fn parse_optional_address(value: &serde_json::Value, name: &str) -> Result<Option<Address>, EngineError> {
    match value.get(name) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => v.as_str().ok_or_else(|| EngineError::Validation(format!("{name} must be a hex address")))?
            .parse().map(Some).map_err(|_| EngineError::Validation(format!("invalid EVM address in {name}"))),
    }
}

fn parse_bytes_field(value: &serde_json::Value, name: &str) -> Result<Option<Bytes>, EngineError> {
    match value.get(name) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => {
            let s = v.as_str().ok_or_else(|| EngineError::Validation(format!("{name} must be a 0x-prefixed hex string")))?;
            let hex = s.strip_prefix("0x").ok_or_else(|| EngineError::Validation(format!("{name} must start with 0x")))?;
            hex::decode(hex).map(Bytes::from).map(Some).map_err(|_| EngineError::Validation(format!("invalid hex in {name}")))
        }
    }
}

fn normalize_raw_balance(raw: &str, decimals: u32) -> Option<String> {
    let value = U256::from_dec_str(raw).ok()?;
    Some(format_units(value, decimals))
}

fn format_units(value: U256, decimals: u32) -> String {
    if decimals == 0 { return value.to_string(); }
    let denom = U256::exp10(decimals as usize);
    let integer = value / denom;
    let remainder = value % denom;
    if remainder.is_zero() { return integer.to_string(); }
    let mut fractional = remainder.to_string();
    while fractional.len() < decimals as usize { fractional.insert(0, '0'); }
    while fractional.ends_with('0') { fractional.pop(); }
    format!("{integer}.{fractional}")
}

impl WalletAdapter for EvmAdapter {
    fn derive_addresses(
        seed: &[u8],
        chains: &[crate::types::chain::ChainRequest],
    ) -> Result<std::collections::HashMap<String, String>, EngineError> {
        // The EVM adapter owns the EVM-family rule: all requested EVM
        // networks use one account derived from the canonical EVM path.
        let address = Self::derive_canonical_address(seed)?;
        let mut out = std::collections::HashMap::with_capacity(chains.len());
        for chain in chains {
            out.insert(chain.name.clone(), address.clone());
        }
        Ok(out)
    }
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 65 || public_key[0] != 0x04 {
            return Err(EngineError::Validation("uncompressed secp256k1 public key expected".into()));
        }
        let mut hasher = Keccak256::new();
        hasher.update(&public_key[1..]);
        let hash = hasher.finalize();
        let address_hex = hex::encode(&hash[12..]);
        let checksum_hash = Keccak256::digest(address_hex.as_bytes());
        let mut checksum = String::with_capacity(42);
        checksum.push_str("0x");
        for (i, ch) in address_hex.chars().enumerate() {
            if ch.is_ascii_alphabetic() && ((checksum_hash[i / 2] >> if i % 2 == 0 { 4 } else { 0 }) & 0x0f) >= 8 {
                checksum.push(ch.to_ascii_uppercase());
            } else {
                checksum.push(ch);
            }
        }
        Ok(checksum)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let key = Self::derive_private_key(seed, path)?;
        let signing_key = SigningKey::from_bytes((&key).into())
            .map_err(|_| EngineError::Crypto("invalid derived EVM private key".into()))?;
        let public = signing_key.verifying_key().to_encoded_point(false);
        Self::pub_key_to_address(public.as_bytes())
    }

    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let key: [u8; 32] = private_key.try_into().map_err(|_| EngineError::Validation("EVM private key must be exactly 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes((&key).into()).map_err(|_| EngineError::Crypto("invalid EVM private key".into()))?;
        let public = signing_key.verifying_key().to_encoded_point(false);
        Self::pub_key_to_address(public.as_bytes())
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()
            .map_err(|e| EngineError::Unexpected(format!("failed to initialize signing runtime: {e}")))?;
        rt.block_on(sign_transaction_json_with_chain(private_key, tx_data, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_known_address_from_private_key() {
        let key = [0x01u8; 32];
        let signing_key = SigningKey::from_bytes((&key).into()).expect("valid key");
        let public = signing_key.verifying_key().to_encoded_point(false);
        let address = EvmAdapter::pub_key_to_address(public.as_bytes()).expect("address");
        assert_eq!(address.len(), 42);
        assert_eq!(address.to_lowercase(), "0x1a642f0e3c3af545e7acbd38b07251b3990914f1");
    }

    #[tokio::test]
    async fn signs_legacy_transaction() {
        let key = [0x01u8; 32];
        let tx = br#"{"type":0,"nonce":"0","to":"0x0000000000000000000000000000000000000001","value":"1","gas":"21000","gas_price":"1000000000","chain_id":1,"data":"0x"}"#;
        let signed = sign_transaction_json_with_chain(&key, tx, 1).await.expect("signing should work");
        assert!(signed.len() > 0);
    }
}
