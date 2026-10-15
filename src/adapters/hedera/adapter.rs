use k256::elliptic_curve::sec1::ToEncodedPoint;
use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key, derive_master_key};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_secp256k1_payload;
use sha3::{Digest, Keccak256};

/// Hedera adapter.
///
/// Initial wallet support intentionally uses Hedera's EVM-compatible account
/// model: secp256k1 + Ethereum-compatible derivation. Hedera-native Ed25519
/// account creation is not part of wallet creation and must not be triggered
/// implicitly by this adapter.
///
/// RPC, account funding/creation transactions, and balance fetching belong to
/// the provider/service layers.
pub struct HederaAdapter;

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

impl HederaAdapter {
    /// Hedera EVM accounts use the Ethereum BIP-44 coin type.
    ///
    /// Expected form:
    /// m/44'/60'/account'/change/address_index
    fn validate_path(path: &str) -> Result<(), EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        if parsed.segments.len() != 5 {
            return Err(EngineError::Validation(
                "Hedera EVM derivation_path must contain five segments, e.g. m/44'/60'/0'/0/0".into(),
            ));
        }
        if parsed.segments[0].index != 44 || !parsed.segments[0].is_hardened
            || parsed.segments[1].index != 60 || !parsed.segments[1].is_hardened
            || !parsed.segments[2].is_hardened
            || parsed.segments[3].is_hardened
            || parsed.segments[4].is_hardened
        {
            return Err(EngineError::Validation(
                "Hedera EVM derivation_path must use m/44'/60'/account'/change/address_index".into(),
            ));
        }
        Ok(())
    }

    fn public_key_from_seed(seed: &[u8], path: &str) -> Result<k256::PublicKey, EngineError> {
        Self::validate_path(path)?;
        let master = derive_master_key(seed)?;
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = derive_child_key(&master, &parsed)?;
        let secret = k256::SecretKey::from_bytes((&child.key).into())
            .map_err(|_| EngineError::Crypto("invalid Hedera secp256k1 key".into()))?;
        Ok(secret.public_key())
    }

    /// Convert an EVM/secp256k1 public key to Hedera's EVM address.
    ///
    /// The address is the rightmost 20 bytes of Keccak-256(uncompressed
    /// public-key coordinates), represented as 0x-prefixed hexadecimal.
    pub fn evm_address_from_public_key(public_key: &[u8]) -> Result<String, EngineError> {
        let public = match public_key.len() {
            33 | 65 => k256::PublicKey::from_sec1_bytes(public_key)
                .map_err(|_| EngineError::Validation("invalid secp256k1 public key".into()))?
                .to_encoded_point(false),
            _ => return Err(EngineError::Validation(
                "Hedera ECDSA public key must be 33 or 65 bytes".into(),
            )),
        };
        let digest = Keccak256::digest(&public.as_bytes()[1..]);
        Ok(format!("0x{}", hex::encode(&digest[12..])))
    }

    /// Normalize provider-returned Hedera HBAR/HTS base-unit amounts.
    /// Fetching remains entirely outside the adapter.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter()
            .map(|mut asset| {
                let decimals = asset.decimals.ok_or_else(|| EngineError::Validation(
                    format!("missing decimals for Hedera asset {}", asset.address)
                ))?;
                asset.balance = normalize_raw_amount(&asset.balance, decimals)?;
                Ok(asset)
            })
            .collect()
    }
}

fn normalize_raw_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
    if raw.trim().is_empty() || raw.starts_with('-') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EngineError::Validation("invalid Hedera base amount".into()));
    }
    let value = raw.trim().parse::<u128>()
        .map_err(|_| EngineError::Validation("Hedera base amount exceeds u128".into()))?;
    if decimals == 0 {
        return Ok(value.to_string());
    }
    let scale = 10u128.checked_pow(decimals)
        .ok_or_else(|| EngineError::Validation("Hedera decimal exponent is too large".into()))?;
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

impl WalletAdapter for HederaAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        Self::evm_address_from_public_key(public_key)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let public_key = Self::public_key_from_seed(seed, path)?;
        Self::evm_address_from_public_key(public_key.to_encoded_point(false).as_bytes())
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Crypto(
                "Hedera ECDSA private key must be 32 bytes".into(),
            ));
        }
        sign_secp256k1_payload(private_key, tx_data)
    }

    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let secret = k256::SecretKey::from_bytes(private_key.into())
            .map_err(|_| EngineError::Crypto("invalid Hedera secp256k1 private key".into()))?;
        Self::evm_address_from_public_key(secret.public_key().to_encoded_point(false).as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hedera_accepts_evm_path() {
        assert!(HederaAdapter::validate_path("m/44'/60'/0'/0/0").is_ok());
        assert!(HederaAdapter::validate_path("m/44'/60'/0'/1/7").is_ok());
    }

    #[test]
    fn hedera_rejects_non_evm_path() {
        assert!(HederaAdapter::validate_path("m/44'/283'/0'/0/0").is_err());
        assert!(HederaAdapter::validate_path("m/44'/60'/0'/0'/0'").is_err());
    }

    #[test]
    fn hedera_address_shape_is_evm_compatible() {
        let address = HederaAdapter::pub_key_to_address(&[2u8; 33]).expect("compressed EVM-compatible public key should derive");
        assert_eq!(address.len(), 42);
        assert!(address.starts_with("0x"));
    }

    #[test]
    fn hedera_normalizes_hbar_units() {
        let assets = vec![AssetRow {
            address: "HBAR".into(),
            symbol: "HBAR".into(),
            decimals: Some(8),
            is_native: true,
            balance: "123456789".into(),
        }];
        let normalized = HederaAdapter::normalize_balances(assets).unwrap();
        assert_eq!(normalized[0].balance, "1.23456789");
    }
}
