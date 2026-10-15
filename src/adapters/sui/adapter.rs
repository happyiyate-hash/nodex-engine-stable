use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key_ed25519, derive_master_key_ed25519};
use crate::crypto::chain_addresses::sui_address_from_public_key;
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_ed25519_payload;

/// Sui wallet adapter.
///
/// Sui is a standalone adapter. It owns Sui key derivation, address
/// construction, transaction signing, and chain-specific balance
/// normalization. RPC/balance fetching remains in the provider layer.
pub struct SuiAdapter;

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

impl SuiAdapter {
    /// Sui uses SLIP-44 coin type 784.
    ///
    /// Standard wallet path:
    /// m/44'/784'/account'/change'/address_index'
    ///
    /// Sui requires an explicit derivation path in the blockchain JSON.
    /// We validate the Sui purpose/coin type and require all components to
    /// be hardened rather than silently substituting a path.
    fn validate_path(path: &str) -> Result<(), EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        if parsed.segments.len() != 5 || parsed.segments.iter().any(|s| !s.is_hardened) {
            return Err(EngineError::Validation(
                "Sui derivation_path must be a five-level hardened path such as m/44'/784'/0'/0'/0'".into(),
            ));
        }
        if parsed.segments[0].index != 44 || parsed.segments[1].index != 784 {
            return Err(EngineError::Validation(
                "Sui derivation_path must use purpose 44' and coin type 784'".into(),
            ));
        }
        Ok(())
    }

    fn derive_public_key(seed: &[u8], path: &str) -> Result<[u8; 32], EngineError> {
        Self::validate_path(path)?;
        let master = derive_master_key_ed25519(seed)?;
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = derive_child_key_ed25519(&master, &parsed)?;

        let secret = ed25519_dalek::SecretKey::from_bytes(&child.key)
            .map_err(|_| EngineError::Crypto("invalid Sui Ed25519 private key".into()))?;
        Ok(ed25519_dalek::PublicKey::from(&secret).to_bytes())
    }

    /// Normalize provider-returned base-unit balances without floating point.
    /// The provider fetches raw values; this function only converts units.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter()
            .map(|mut asset| {
                if let Some(decimals) = asset.decimals {
                    asset.balance = normalize_base_amount(&asset.balance, decimals)?;
                }
                Ok(asset)
            })
            .collect()
    }

    fn transaction_digest(tx_data: &[u8]) -> [u8; 32] {
        // Sui intent message for TransactionData:
        // scope = Transaction (0), application = Sui (0), version = V0 (0).
        // tx_data must be canonical BCS-serialized TransactionData.
        let mut intent_message = Vec::with_capacity(3 + tx_data.len());
        intent_message.extend_from_slice(&[0, 0, 0]);
        intent_message.extend_from_slice(tx_data);
        blake2b_256(&intent_message)
    }
}

fn blake2b_256(data: &[u8]) -> [u8; 32] {
    use blake2::{Blake2b512, Digest};
    let digest = Blake2b512::digest(data);
    digest[..32].try_into().expect("Blake2b-512 digest is 64 bytes")
}

fn normalize_base_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
    if raw.trim().is_empty() || raw.starts_with('-') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EngineError::Validation("invalid Sui base amount".into()));
    }

    let value = raw.trim().parse::<u128>()
        .map_err(|_| EngineError::Validation("Sui base amount exceeds u128".into()))?;

    if decimals == 0 {
        return Ok(value.to_string());
    }

    let scale = 10u128.checked_pow(decimals)
        .ok_or_else(|| EngineError::Validation("Sui decimal exponent is too large".into()))?;

    let whole = value / scale;
    let fraction = value % scale;
    if fraction == 0 {
        return Ok(whole.to_string());
    }

    let mut fraction_text = format!("{:0width$}", fraction, width = decimals as usize);
    while fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    Ok(format!("{}.{}", whole, fraction_text))
}

impl WalletAdapter for SuiAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        sui_address_from_public_key(public_key)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let public_key = Self::derive_public_key(seed, path)?;
        sui_address_from_public_key(&public_key)
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Crypto(
                "Sui Ed25519 private key must be exactly 32 bytes".into(),
            ));
        }

        let digest = Self::transaction_digest(tx_data);
        let signature = sign_ed25519_payload(private_key, &digest)?;
        if signature.len() != 64 {
            return Err(EngineError::Crypto(
                "Sui Ed25519 signer returned an invalid signature length".into(),
            ));
        }

        let secret = ed25519_dalek::SecretKey::from_bytes(private_key)
            .map_err(|_| EngineError::Crypto("invalid Sui Ed25519 private key".into()))?;
        let public_key = ed25519_dalek::PublicKey::from(&secret);

        // Sui serialized Ed25519 signature:
        // scheme flag (0x00) || 64-byte signature || 32-byte public key.
        let mut sui_signature = Vec::with_capacity(97);
        sui_signature.push(0x00);
        sui_signature.extend_from_slice(&signature);
        sui_signature.extend_from_slice(public_key.as_bytes());
        Ok(sui_signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sui_requires_standard_hardened_path() {
        assert!(SuiAdapter::validate_path("m/44'/784'/0'/0'/0'").is_ok());
        assert!(SuiAdapter::validate_path("m/44'/60'/0'/0'/0'").is_err());
        assert!(SuiAdapter::validate_path("m/44'/784'/0'/0/0").is_err());
    }

    #[test]
    fn normalizes_sui_exactly() {
        let assets = vec![AssetRow {
            address: "0x2::sui::SUI".into(),
            symbol: "SUI".into(),
            decimals: Some(9),
            is_native: true,
            balance: "1234567890".into(),
        }];
        let normalized = SuiAdapter::normalize_balances(assets).unwrap();
        assert_eq!(normalized[0].balance, "1.23456789");
    }

    #[test]
    fn rejects_invalid_base_amount() {
        let assets = vec![AssetRow {
            address: "0x2::sui::SUI".into(),
            symbol: "SUI".into(),
            decimals: Some(9),
            is_native: true,
            balance: "not-a-number".into(),
        }];
        assert!(SuiAdapter::normalize_balances(assets).is_err());
    }

    #[test]
    fn sui_transaction_digest_is_32_bytes() {
        assert_eq!(SuiAdapter::transaction_digest(b"test").len(), 32);
    }
}
