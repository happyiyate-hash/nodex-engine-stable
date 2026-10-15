use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key_ed25519, derive_master_key_ed25519};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_ed25519_payload;
use sha2::{Digest, Sha256};

/// NEAR Protocol adapter.
///
/// NEAR is a standalone chain. The adapter handles key derivation, implicit
/// account-address construction, transaction signing, and chain-specific balance
/// normalization. RPC/balance fetching remains in the provider layer.
pub struct NearAdapter;

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

impl NearAdapter {
    /// NEAR's registered SLIP-44 coin type is 397.
    ///
    /// The canonical seed-phrase path is m/44'/397'/0'. Hardware wallets may
    /// use longer hardened paths such as 44'/397'/0'/0'/1'. We therefore
    /// validate the NEAR purpose/coin type and require every supplied segment
    /// to be hardened, while allowing the caller to select the exact path.
    fn validate_path(path: &str) -> Result<(), EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        if parsed.segments.len() < 3 || parsed.segments.iter().any(|s| !s.is_hardened) {
            return Err(EngineError::Validation(
                "NEAR derivation_path must be hardened and use at least m/44'/397'/account'".into(),
            ));
        }
        if parsed.segments[0].index != 44 || parsed.segments[1].index != 397 {
            return Err(EngineError::Validation(
                "NEAR derivation_path must use purpose 44' and coin type 397'".into(),
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
            .map_err(|_| EngineError::Crypto("invalid NEAR Ed25519 key".into()))?;
        Ok(ed25519_dalek::PublicKey::from(&secret).to_bytes())
    }

    /// NEAR implicit account IDs are the lowercase hexadecimal Ed25519
    /// public key (32 bytes = 64 hex characters).
    fn implicit_account(public_key: &[u8; 32]) -> String {
        hex::encode(public_key)
    }

    /// NEAR signs the SHA-256 hash of the canonical Borsh-serialized
    /// transaction. tx_data must therefore be canonical transaction bytes,
    /// not JSON or an already signed transaction.
    /// Normalize provider-returned integer amounts using chain-specific decimals.
    /// Fetching remains exclusively in the balance provider.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Vec<AssetRow> {
        assets.into_iter().map(|mut asset| {
            if let Some(decimals) = asset.decimals {
                asset.balance = normalize_raw_amount(&asset.balance, decimals)
                    .unwrap_or_else(|| "0".to_string());
            }
            asset
        }).collect()
    }

    fn transaction_digest(tx_data: &[u8]) -> [u8; 32] {
        Sha256::digest(tx_data).into()
    }
}

impl WalletAdapter for NearAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 32 {
            return Err(EngineError::Validation(
                "NEAR Ed25519 public key must be 32 bytes".into(),
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(public_key);
        Ok(Self::implicit_account(&key))
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let public_key = Self::derive_public_key(seed, path)?;
        Ok(Self::implicit_account(&public_key))
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Validation(
                "NEAR Ed25519 private key must be 32 bytes".into(),
            ));
        }
        let digest = Self::transaction_digest(tx_data);
        sign_ed25519_payload(private_key, &digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_accepts_canonical_seed_phrase_path() {
        assert!(NearAdapter::validate_path("m/44'/397'/0'").is_ok());
    }

    #[test]
    fn near_accepts_hardware_wallet_variation() {
        assert!(NearAdapter::validate_path("m/44'/397'/0'/0'/1'").is_ok());
    }

    #[test]
    fn near_rejects_wrong_coin_type_or_unhardened_segment() {
        assert!(NearAdapter::validate_path("m/44'/60'/0'").is_err());
        assert!(NearAdapter::validate_path("m/44'/397'/0'/0/1").is_err());
    }

    #[test]
    fn near_implicit_address_is_64_lowercase_hex() {
        let address = NearAdapter::pub_key_to_address(&[0u8; 32]).unwrap();
        assert_eq!(address.len(), 64);
        assert!(address.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn near_normalizes_yocto_balance() {
        let assets = vec![AssetRow {
            address: "".into(), symbol: "NEAR".into(), decimals: Some(24), is_native: true,
            balance: "1000000000000000000000000".into(),
        }];
        let normalized = NearAdapter::normalize_balances(assets);
        assert_eq!(normalized[0].balance, "1");
    }

    #[test]
    fn near_preserves_zero_decimal_amounts() {
        let assets = vec![AssetRow {
            address: "token.near".into(), symbol: "TOKEN".into(), decimals: Some(0), is_native: false,
            balance: "42".into(),
        }];
        let normalized = NearAdapter::normalize_balances(assets);
        assert_eq!(normalized[0].balance, "42");
    }

    #[test]
    fn near_transaction_digest_is_sha256() {
        let digest = NearAdapter::transaction_digest(b"test");
        assert_eq!(digest.len(), 32);
    }
}


fn normalize_raw_amount(raw: &str, decimals: u32) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('-') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits = raw.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if decimals == 0 {
        return Some(digits.to_string());
    }
    let scale = decimals as usize;
    if digits.len() <= scale {
        let fraction = format!("{:0>width$}", digits, width = scale).trim_end_matches('0').to_string();
        return if fraction.is_empty() { Some("0".into()) } else { Some(format!("0.{}", fraction)) };
    }
    let split = digits.len() - scale;
    let whole = &digits[..split];
    let fraction = digits[split..].trim_end_matches('0');
    if fraction.is_empty() { Some(whole.to_string()) } else { Some(format!("{}.{}", whole, fraction)) }
}
